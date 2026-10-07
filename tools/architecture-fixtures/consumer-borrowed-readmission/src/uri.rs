//! Byte-resumable typed URI resolution. Only immutable external spans and
//! scalar positions survive suspension. The existing TD resolver is the
//! differential oracle; it is never called inside bounded progress.
use super::{
    Cause, Job, Trace, pay,
    storage::{Site, Storage},
};
use crate::data_type::{BaseUri, FormHref};
use clinkz_wot_foundation::{ResourceKind as R, WorkBudget, WorkClass as W};
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Observations {
    pub component_bytes: u64,
    pub merge_bytes: u64,
    pub segment_bytes: u64,
    pub pop_bytes: u64,
    pub emitted_bytes: u64,
    pub shifted_bytes: u64,
    pub utf8_bytes: u64,
}
#[derive(Clone, Copy, Default)]
struct Parts {
    scheme: Option<usize>,
    authority: Option<(usize, usize)>,
    path: (usize, usize),
    query: Option<(usize, usize)>,
    fragment: Option<(usize, usize)>,
}
struct Scan {
    position: usize,
    slash: usize,
    query: usize,
    fragment: usize,
    scheme: Option<usize>,
    stage: u8,
    authority_start: usize,
    authority_end: usize,
}
impl Scan {
    fn new(len: usize) -> Self {
        Self {
            position: 0,
            slash: len,
            query: len,
            fragment: len,
            scheme: None,
            stage: 0,
            authority_start: 0,
            authority_end: 0,
        }
    }
    fn tick(&mut self, text: &str, observed: &mut Observations) -> Option<Parts> {
        if self.stage == 0 && self.position < text.len() {
            let n = self.position;
            let c = text.as_bytes()[n];
            self.position += 1;
            observed.component_bytes += 1;
            if c == b'/' && self.slash == text.len() {
                self.slash = n;
            }
            if c == b'?' && self.query == text.len() && n < self.fragment {
                self.query = n;
            }
            if c == b'#' && self.fragment == text.len() {
                self.fragment = n;
            }
            if c == b':'
                && self.scheme.is_none()
                && n < self.slash
                && n < self.query
                && n < self.fragment
            {
                self.scheme = Some(n);
            }
            return None;
        }
        let anchor = self.scheme.map_or(0, |n| n + 1);
        let end = self.query.min(self.fragment);
        if self.stage == 0 {
            self.stage = 1;
            self.position = anchor;
        }
        if self.stage == 1 {
            // Both reads are made under the two-byte component debit.
            let authority = end >= anchor + 2 && &text.as_bytes()[anchor..anchor + 2] == b"//";
            observed.component_bytes += if end >= anchor + 2 { 2 } else { 0 };
            self.authority_start = anchor + 2;
            self.authority_end = if authority { anchor + 2 } else { anchor };
            self.stage = if authority { 2 } else { 3 };
            return None;
        }
        if self.stage == 2 {
            if self.authority_end < end {
                let c = text.as_bytes()[self.authority_end];
                observed.component_bytes += 1;
                if c == b'/' {
                    self.stage = 3;
                } else {
                    self.authority_end += 1;
                }
                return None;
            }
            self.stage = 3;
        }
        Some(Parts {
            scheme: self.scheme,
            authority: (self.authority_end >= self.authority_start)
                .then_some((self.authority_start, self.authority_end)),
            path: (self.authority_end, end),
            query: (self.query < self.fragment).then_some((self.query + 1, self.fragment)),
            fragment: (self.fragment < text.len()).then_some((self.fragment + 1, text.len())),
        })
    }
}
#[derive(Clone, Copy, Default)]
struct Span {
    source: u8,
    start: usize,
    end: usize,
}
// source 0=base, 1=href, 2=fixed punctuation. No span borrows output storage.
const PUNCTUATION: &str = "://?#";
fn span(source: u8, range: (usize, usize)) -> Span {
    Span {
        source,
        start: range.0,
        end: range.1,
    }
}
fn classify(bytes: &[u8]) -> u8 {
    if matches!(bytes, b"." | b"%2E" | b"%2e") {
        1
    } else if matches!(
        bytes,
        b".."
            | b".%2E"
            | b".%2e"
            | b"%2E."
            | b"%2e."
            | b"%2E%2E"
            | b"%2E%2e"
            | b"%2e%2E"
            | b"%2e%2e"
    ) {
        2
    } else {
        0
    }
}
pub struct Resolver<'a> {
    base: Option<&'a BaseUri>,
    href: &'a FormHref,
    ceiling: usize,
    stage: u8,
    scan: Scan,
    reference: Parts,
    base_parts: Parts,
    prefix: [Span; 4],
    prefix_count: usize,
    piece: usize,
    position: usize,
    paths: [Span; 2],
    path_count: usize,
    path_piece: usize,
    path_start: usize,
    segment_start: usize,
    segment_end: usize,
    segment: [u8; 6],
    segment_len: usize,
    merge: usize,
    merge_last: [u8; 6],
    merge_len: usize,
    pop: usize,
    query: Option<Span>,
    fragment: Option<Span>,
    suffix: [Span; 4],
    suffix_count: usize,
    alias: bool,
    absolute_path: bool,
    valid: bool,
    utf8_left: u8,
    utf8_min: u8,
    utf8_max: u8,
    observed: Observations,
}
impl<'a> Resolver<'a> {
    pub fn new(base: Option<&'a BaseUri>, href: &'a FormHref, ceiling: usize) -> Self {
        Self {
            base,
            href,
            ceiling,
            stage: 0,
            scan: Scan::new(href.as_str().len()),
            reference: Parts::default(),
            base_parts: Parts::default(),
            prefix: [Span::default(); 4],
            prefix_count: 0,
            piece: 0,
            position: 0,
            paths: [Span::default(); 2],
            path_count: 0,
            path_piece: 0,
            path_start: 0,
            segment_start: 0,
            segment_end: 0,
            segment: [0; 6],
            segment_len: 0,
            merge: 0,
            merge_last: [0; 6],
            merge_len: 0,
            pop: 0,
            query: None,
            fragment: None,
            suffix: [Span::default(); 4],
            suffix_count: 0,
            alias: false,
            absolute_path: false,
            valid: false,
            utf8_left: 0,
            utf8_min: 0x80,
            utf8_max: 0xbf,
            observed: Observations::default(),
        }
    }
    fn text(&self, source: u8) -> &str {
        match source {
            0 => self.base.unwrap().as_str(),
            1 => self.href.as_str(),
            _ => PUNCTUATION,
        }
    }
    pub fn observations(&self) -> Observations {
        self.observed
    }
    pub fn is_alias(&self) -> bool {
        self.alias
    }
    pub fn complete(&self) -> bool {
        self.stage == 20
    }
    pub fn result<'s, F>(&'s self, storage: &'s Storage<F>) -> &'s str {
        assert!(self.complete() && self.valid);
        if self.alias {
            self.href.as_str()
        } else {
            // SAFETY: the incremental validator completed on this exact immutable
            // byte prefix. A result loan prevents another poll/move/reset.
            unsafe { core::str::from_utf8_unchecked(storage.temporary_bytes()) }
        }
    }
    fn output(&mut self, storage: &mut Storage<Job<'a>>, byte: u8) -> Result<(), Cause> {
        if storage.len(Site::Bytes) == self.ceiling {
            return Err(Cause::Resource {
                kind: R::UriTemplateSourceBytesMax,
                configured: self.ceiling as u64,
                observed: self.ceiling as u64 + 1,
            });
        }
        storage.push_byte(byte);
        self.observed.emitted_bytes += 1;
        Ok(())
    }
    fn add_prefix(&mut self, s: Span) {
        self.prefix[self.prefix_count] = s;
        self.prefix_count += 1;
    }
    fn suffixes(&mut self) {
        self.suffix_count = 0;
        if let Some(q) = self.query {
            self.suffix[0] = span(2, (3, 4));
            self.suffix[1] = q;
            self.suffix_count = 2;
        }
        if let Some(f) = self.fragment {
            self.suffix[self.suffix_count] = span(2, (4, 5));
            self.suffix[self.suffix_count + 1] = f;
            self.suffix_count += 2;
        }
        self.piece = 0;
        self.position = 0;
        self.stage = 13;
    }
    pub fn step(
        &mut self,
        storage: &mut Storage<Job<'a>>,
        budget: &mut WorkBudget,
        lifetime: &mut u64,
        trace: &mut Trace,
    ) -> Result<bool, Cause> {
        let mut actions = 0;
        while !self.complete() {
            if actions == 1 {
                return Ok(false);
            }
            actions += 1;
            let costs = match self.stage {
                0 | 1 => [
                    (W::UriBytes, 2),
                    (W::CodecOutputBytes, 0),
                    (W::CleanupItems, 0),
                ],
                4 => [
                    (W::UriBytes, 1),
                    (W::CodecOutputBytes, 0),
                    (
                        W::CleanupItems,
                        u64::from(storage.capacity(Site::Bytes) == 0 && self.ceiling != 0),
                    ),
                ],
                5 | 8 | 13 => [
                    (W::UriBytes, 2),
                    (W::CodecOutputBytes, 1),
                    (W::CleanupItems, 0),
                ],
                16 | 17 => [
                    (W::UriBytes, 4),
                    (W::CodecOutputBytes, 2),
                    (W::CleanupItems, 0),
                ],
                2 | 11 => [
                    (W::UriBytes, 2),
                    (W::CodecOutputBytes, 0),
                    (W::CleanupItems, 0),
                ],
                _ => [
                    (W::UriBytes, 1),
                    (W::CodecOutputBytes, 0),
                    (W::CleanupItems, 0),
                ],
            };
            if !pay(budget, lifetime, trace, &costs)? {
                return Ok(false);
            }
            match self.stage {
                0 => {
                    if self.href.as_str().len() > self.ceiling {
                        return Err(Cause::Resource {
                            kind: R::UriTemplateSourceBytesMax,
                            configured: self.ceiling as u64,
                            observed: self.href.as_str().len() as u64,
                        });
                    }
                    if self.href.is_template() {
                        self.alias = true;
                        self.valid = true;
                        self.stage = 20;
                        continue;
                    }
                    if let Some(parts) = self.scan.tick(self.href.as_str(), &mut self.observed) {
                        self.reference = parts;
                        if parts.scheme.is_some() || self.base.is_none() {
                            self.alias = true;
                            self.valid = true;
                            self.stage = 20;
                        } else {
                            if self.base.unwrap().is_template() {
                                return Err(Cause::Uri);
                            }
                            self.scan = Scan::new(self.base.unwrap().as_str().len());
                            self.stage = 1;
                        }
                    }
                }
                1 => {
                    let text = self.base.unwrap().as_str();
                    if let Some(parts) = self.scan.tick(text, &mut self.observed) {
                        self.base_parts = parts;
                        self.stage = 2;
                    }
                }
                2 => {
                    let b = self.base_parts;
                    let r = self.reference;
                    if b.fragment.is_some() {
                        return Err(Cause::Uri);
                    }
                    if b.authority.is_none()
                        && b.path.0 < b.path.1
                        && self.text(0).as_bytes()[b.path.0] != b'/'
                        && !matches!(self.href.as_str().as_bytes().first(), None | Some(b'#'))
                    {
                        return Err(Cause::Uri);
                    }
                    self.add_prefix(span(0, (0, b.scheme.ok_or(Cause::Uri)?)));
                    self.add_prefix(span(2, (0, 1)));
                    if let Some((source, a)) = r
                        .authority
                        .map(|v| (1, v))
                        .or_else(|| b.authority.map(|v| (0, v)))
                    {
                        self.add_prefix(span(2, (1, 3)));
                        self.add_prefix(span(source, a));
                    }
                    self.fragment = r.fragment.map(|v| span(1, v));
                    if r.authority.is_some() {
                        self.paths[0] = span(1, r.path);
                        self.path_count = 1;
                        self.query = r.query.map(|v| span(1, v));
                        self.stage = 4;
                    } else if r.path.0 == r.path.1 {
                        self.paths[0] = span(0, b.path);
                        self.path_count = 1;
                        self.query = r
                            .query
                            .map(|v| span(1, v))
                            .or_else(|| b.query.map(|v| span(0, v)));
                        self.stage = 4;
                    } else if self.text(1).as_bytes()[r.path.0] == b'/' {
                        self.paths[0] = span(1, r.path);
                        self.path_count = 1;
                        self.query = r.query.map(|v| span(1, v));
                        self.stage = 4;
                    } else {
                        self.query = r.query.map(|v| span(1, v));
                        self.merge = b.path.1;
                        self.merge_len = 0;
                        self.stage = 3;
                    }
                }
                3 => {
                    let b = self.base_parts;
                    if b.path.0 == b.path.1 {
                        self.paths[0] = span(2, (1, 2));
                        self.paths[1] = span(1, self.reference.path);
                        self.path_count = 2;
                        self.stage = 4;
                    } else if self.merge > b.path.0 {
                        self.merge -= 1;
                        let c = self.text(0).as_bytes()[self.merge];
                        self.observed.merge_bytes += 1;
                        if c == b'/' {
                            let mut tail = [0; 6];
                            if self.merge_len <= 6 {
                                for n in 0..self.merge_len {
                                    tail[n] = self.merge_last[self.merge_len - 1 - n];
                                }
                            }
                            let end =
                                if self.merge_len <= 6 && classify(&tail[..self.merge_len]) == 2 {
                                    b.path.1
                                } else {
                                    self.merge + 1
                                };
                            self.paths[0] = span(0, (b.path.0, end));
                            self.paths[1] = span(1, self.reference.path);
                            self.path_count = 2;
                            self.stage = 4;
                        } else {
                            if self.merge_len < 6 {
                                self.merge_last[self.merge_len] = c;
                            }
                            self.merge_len += 1;
                        }
                    } else {
                        return Err(Cause::Uri);
                    }
                }
                4 => {
                    if storage.capacity(Site::Bytes) == 0 && self.ceiling != 0 {
                        storage.begin_grow(Site::Bytes, self.ceiling)?;
                    }
                    storage.truncate_bytes(0);
                    self.piece = 0;
                    self.position = 0;
                    self.stage = 5;
                }
                5 => {
                    if self.piece < self.prefix_count {
                        let s = self.prefix[self.piece];
                        if self.position < s.end - s.start {
                            let byte = self.text(s.source).as_bytes()[s.start + self.position];
                            self.output(storage, byte)?;
                            self.position += 1;
                        } else {
                            self.piece += 1;
                            self.position = 0;
                        }
                    } else {
                        self.path_start = storage.len(Site::Bytes);
                        self.path_piece = 0;
                        self.position = self.paths[0].start;
                        self.absolute_path = self.paths[0].start < self.paths[0].end
                            && self.text(self.paths[0].source).as_bytes()[self.paths[0].start]
                                == b'/';
                        self.segment_start = self.position;
                        self.segment_end = self.position;
                        self.segment_len = 0;
                        self.stage = 6;
                    }
                }
                6 => {
                    if self.path_piece == self.path_count {
                        self.stage = 11;
                        continue;
                    }
                    let s = self.paths[self.path_piece];
                    if self.segment_end == s.end {
                        if self.segment_start < self.segment_end {
                            self.position = self.segment_start;
                            self.stage = 7;
                        } else {
                            self.path_piece += 1;
                            if self.path_piece < self.path_count {
                                self.segment_start = self.paths[self.path_piece].start;
                                self.segment_end = self.segment_start;
                                self.segment_len = 0;
                            }
                        }
                    } else {
                        let byte = self.text(s.source).as_bytes()[self.segment_end];
                        self.segment_end += 1;
                        self.observed.segment_bytes += 1;
                        if byte == b'/' {
                            self.position = self.segment_start;
                            self.stage = 7;
                        } else {
                            if self.segment_len < 6 {
                                self.segment[self.segment_len] = byte;
                            }
                            self.segment_len += 1;
                        }
                    }
                }
                7 => {
                    let kind = if self.absolute_path && self.segment_len <= 6 {
                        classify(&self.segment[..self.segment_len])
                    } else {
                        0
                    };
                    match kind {
                        0 => self.stage = 8,
                        1 => {
                            self.segment_start = self.segment_end;
                            self.segment_len = 0;
                            self.stage = 6;
                        }
                        _ => {
                            self.pop = storage.len(Site::Bytes).saturating_sub(1);
                            self.stage = 9;
                        }
                    }
                }
                8 => {
                    if self.position < self.segment_end {
                        let s = self.paths[self.path_piece];
                        let byte = self.text(s.source).as_bytes()[self.position];
                        self.output(storage, byte)?;
                        self.position += 1;
                    } else {
                        self.segment_start = self.segment_end;
                        self.segment_len = 0;
                        self.stage = 6;
                    }
                }
                9 => {
                    if storage.len(Site::Bytes) > self.path_start + 1 && self.pop > self.path_start
                    {
                        self.pop -= 1;
                        let byte = storage.byte(self.pop);
                        self.observed.pop_bytes += 1;
                        if byte == b'/' {
                            storage.truncate_bytes(self.pop + 1);
                            self.segment_start = self.segment_end;
                            self.segment_len = 0;
                            self.stage = 6;
                        }
                    } else {
                        self.segment_start = self.segment_end;
                        self.segment_len = 0;
                        self.stage = 6;
                    }
                }
                11 => {
                    let authority =
                        self.reference.authority.is_some() || self.base_parts.authority.is_some();
                    if !authority
                        && storage.len(Site::Bytes) >= self.path_start + 2
                        && storage.byte(self.path_start) == b'/'
                        && storage.byte(self.path_start + 1) == b'/'
                    {
                        if storage.len(Site::Bytes) + 2 > self.ceiling {
                            return Err(Cause::Resource {
                                kind: R::UriTemplateSourceBytesMax,
                                configured: self.ceiling as u64,
                                observed: (storage.len(Site::Bytes) + 2) as u64,
                            });
                        }
                        self.pop = storage.len(Site::Bytes);
                        self.stage = 16;
                    } else {
                        self.suffixes();
                    }
                }
                16 => {
                    self.output(storage, 0)?;
                    self.output(storage, 0)?;
                    self.stage = 17;
                }
                17 => {
                    // A shift touches one old byte, one initialized destination.
                    if self.pop > self.path_start {
                        self.pop -= 1;
                        let byte = storage.byte(self.pop);
                        // Destination already initialized by the two paid appends.
                        let destination = self.pop + 2;
                        storage.set_byte(destination, byte);
                        self.observed.shifted_bytes += 1;
                    } else {
                        storage.set_byte(self.path_start, b'/');
                        storage.set_byte(self.path_start + 1, b'.');
                        self.suffixes();
                    }
                }
                13 => {
                    if self.piece < self.suffix_count {
                        let s = self.suffix[self.piece];
                        if self.position < s.end - s.start {
                            let byte = self.text(s.source).as_bytes()[s.start + self.position];
                            self.output(storage, byte)?;
                            self.position += 1;
                        } else {
                            self.piece += 1;
                            self.position = 0;
                        }
                    } else {
                        self.position = 0;
                        self.stage = 14;
                    }
                }
                14 => {
                    if self.position == storage.len(Site::Bytes) {
                        if self.utf8_left != 0 {
                            return Err(Cause::Uri);
                        }
                        self.valid = true;
                        self.stage = 20;
                    } else {
                        let byte = storage.byte(self.position);
                        self.position += 1;
                        self.observed.utf8_bytes += 1;
                        if self.utf8_left != 0 {
                            if byte < self.utf8_min || byte > self.utf8_max {
                                return Err(Cause::Uri);
                            }
                            self.utf8_left -= 1;
                            self.utf8_min = 0x80;
                            self.utf8_max = 0xbf;
                        } else {
                            let (left, min, max) = match byte {
                                0..=0x7f => (0, 0x80, 0xbf),
                                0xc2..=0xdf => (1, 0x80, 0xbf),
                                0xe0 => (2, 0xa0, 0xbf),
                                0xe1..=0xec | 0xee..=0xef => (2, 0x80, 0xbf),
                                0xed => (2, 0x80, 0x9f),
                                0xf0 => (3, 0x90, 0xbf),
                                0xf1..=0xf3 => (3, 0x80, 0xbf),
                                0xf4 => (3, 0x80, 0x8f),
                                _ => return Err(Cause::Uri),
                            };
                            self.utf8_left = left;
                            self.utf8_min = min;
                            self.utf8_max = max;
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(true)
    }
}
