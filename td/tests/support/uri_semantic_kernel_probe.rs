//! Test-only storage-neutral URI-resolution kernel.
//!
//! The current production helper owns its parsed and resolved results. This
//! prototype instead accepts already classified borrowed TD values and writes
//! a composite RFC 3986 result into caller-owned storage. Both the typed Thing
//! adapter and the three-arena Snapshot adapter call this one implementation.

use crate::data_type::{BaseUri, FormHref};
use fluent_uri::{Uri, UriRef};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BorrowedBase<'a> {
    Absolute(&'a str),
    Template(&'a str),
}

impl<'a> BorrowedBase<'a> {
    pub(super) fn from_typed(base: &'a BaseUri) -> Self {
        match base {
            BaseUri::Absolute(base) => Self::Absolute(base.as_str()),
            BaseUri::Template(base) => Self::Template(base),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BorrowedHref<'a> {
    Reference(&'a str),
    Template(&'a str),
}

impl<'a> BorrowedHref<'a> {
    pub(super) fn from_typed(href: &'a FormHref) -> Self {
        match href {
            FormHref::Reference(reference) => Self::Reference(reference.as_str()),
            FormHref::Template(template) => Self::Template(template),
        }
    }

    pub(super) const fn as_str(self) -> &'a str {
        match self {
            Self::Reference(value) | Self::Template(value) => value,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResolveInto {
    /// The resolved value is byte-for-byte the raw href, so storage may alias it.
    AliasRaw,
    /// A composite reference was written into the supplied byte sink.
    Written,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ResolveIntoError {
    TemplateBase,
    Resolution,
    Capacity,
}

/// Mutable output needed by RFC 3986 path merging and dot-segment removal.
/// Implementations may be an inline test buffer or the existing byte arena.
pub(super) trait UriOutput {
    fn len(&self) -> usize;
    fn bytes(&self) -> &[u8];
    fn push_str(&mut self, value: &str) -> Result<(), ResolveIntoError>;
    fn push_byte(&mut self, value: u8) -> Result<(), ResolveIntoError>;
    fn truncate(&mut self, length: usize);
    fn insert_str(&mut self, index: usize, value: &str) -> Result<(), ResolveIntoError>;
}

/// Applies today's TD target semantics without owning the output.
///
/// Template hrefs, absolute hrefs, and relative hrefs without a base alias the
/// raw input. Only a relative reference against a concrete base is emitted.
/// This keeps the future borrowed view honest: any composite `&str` must be
/// retained by its owner before Planning asks for it.
pub(super) fn resolve_form_href_into(
    base: Option<BorrowedBase<'_>>,
    href: BorrowedHref<'_>,
    output: &mut impl UriOutput,
) -> Result<ResolveInto, ResolveIntoError> {
    let BorrowedHref::Reference(reference_text) = href else {
        return Ok(ResolveInto::AliasRaw);
    };
    let reference = UriRef::parse(reference_text).map_err(|_| ResolveIntoError::Resolution)?;

    if reference.has_scheme() || base.is_none() {
        return Ok(ResolveInto::AliasRaw);
    }

    let base_text = match base.unwrap() {
        BorrowedBase::Absolute(base) => base,
        BorrowedBase::Template(_) => return Err(ResolveIntoError::TemplateBase),
    };
    let base = Uri::parse(base_text).map_err(|_| ResolveIntoError::Resolution)?;
    resolve_relative(&base, &reference, output)?;
    Ok(ResolveInto::Written)
}

fn resolve_relative(
    base: &Uri<&str>,
    reference: &UriRef<&str>,
    output: &mut impl UriOutput,
) -> Result<(), ResolveIntoError> {
    if base.has_fragment() {
        return Err(ResolveIntoError::Resolution);
    }
    if !base.has_authority()
        && base.path().is_rootless()
        && !matches!(reference.as_str().as_bytes().first(), None | Some(b'#'))
    {
        return Err(ResolveIntoError::Resolution);
    }

    output.push_str(base.scheme().as_str())?;
    output.push_byte(b':')?;

    let reference_authority = reference.authority();
    let authority = reference_authority.or_else(|| base.authority());
    if let Some(authority) = authority {
        output.push_str("//")?;
        output.push_str(authority.as_str())?;
    }

    let path_start = output.len();
    let reference_path = reference.path();
    let mut first_path = reference_path.as_str();
    let mut second_path = None;
    let query;

    if reference_authority.is_some() {
        query = reference.query();
    } else if reference_path.is_empty() {
        first_path = base.path().as_str();
        query = reference.query().or_else(|| base.query());
    } else {
        if !reference_path.is_absolute() {
            let base_path = base.path().as_str();
            let base_path = if base_path.is_empty() { "/" } else { base_path };
            let last_slash = base_path
                .rfind('/')
                .expect("absolute base path has a merge point");
            let last_segment = &base_path[last_slash + 1..];
            first_path = if classify_segment(last_segment) == SegmentKind::DoubleDot {
                base_path
            } else {
                &base_path[..=last_slash]
            };
            second_path = Some(reference_path.as_str());
        }
        query = reference.query();
    }

    if first_path.starts_with('/') {
        remove_dot_segments(output, path_start, first_path)?;
        if let Some(path) = second_path {
            remove_dot_segments(output, path_start, path)?;
        }
    } else {
        output.push_str(first_path)?;
        if let Some(path) = second_path {
            output.push_str(path)?;
        }
    }

    if authority.is_none() && output.bytes()[path_start..].starts_with(b"//") {
        output.insert_str(path_start, "/.")?;
    }

    if let Some(query) = query {
        output.push_byte(b'?')?;
        output.push_str(query.as_str())?;
    }
    if let Some(fragment) = reference.fragment() {
        output.push_byte(b'#')?;
        output.push_str(fragment.as_str())?;
    }
    Ok(())
}

fn remove_dot_segments(
    output: &mut impl UriOutput,
    path_start: usize,
    path: &str,
) -> Result<(), ResolveIntoError> {
    for segment in path.split_inclusive('/') {
        let stripped = segment.strip_suffix('/').unwrap_or(segment);
        match classify_segment(stripped) {
            SegmentKind::Dot => {}
            SegmentKind::DoubleDot => {
                if output.len() > path_start + 1 {
                    let end = output.len() - 1;
                    let slash = output.bytes()[path_start..end]
                        .iter()
                        .rposition(|byte| *byte == b'/')
                        .map(|index| path_start + index)
                        .expect("absolute emitted path retains a slash");
                    output.truncate(slash + 1);
                }
            }
            SegmentKind::Normal => output.push_str(segment)?,
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SegmentKind {
    Dot,
    DoubleDot,
    Normal,
}

fn classify_segment(mut segment: &str) -> SegmentKind {
    if segment.is_empty() {
        return SegmentKind::Normal;
    }
    if let Some(remainder) = segment.strip_prefix('.') {
        segment = remainder;
    } else if let Some(remainder) = segment.strip_prefix("%2E") {
        segment = remainder;
    } else if let Some(remainder) = segment.strip_prefix("%2e") {
        segment = remainder;
    }
    if segment.is_empty() {
        SegmentKind::Dot
    } else if matches!(segment, "." | "%2E" | "%2e") {
        SegmentKind::DoubleDot
    } else {
        SegmentKind::Normal
    }
}

pub(super) struct FixedUriBuffer<const N: usize> {
    bytes: [u8; N],
    length: usize,
}

impl<const N: usize> FixedUriBuffer<N> {
    pub(super) const fn new() -> Self {
        Self {
            bytes: [0; N],
            length: 0,
        }
    }

    pub(super) fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.length]).unwrap()
    }
}

impl<const N: usize> UriOutput for FixedUriBuffer<N> {
    fn len(&self) -> usize {
        self.length
    }

    fn bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }

    fn push_str(&mut self, value: &str) -> Result<(), ResolveIntoError> {
        let end = self
            .length
            .checked_add(value.len())
            .ok_or(ResolveIntoError::Capacity)?;
        if end > N {
            return Err(ResolveIntoError::Capacity);
        }
        self.bytes[self.length..end].copy_from_slice(value.as_bytes());
        self.length = end;
        Ok(())
    }

    fn push_byte(&mut self, value: u8) -> Result<(), ResolveIntoError> {
        if self.length == N {
            return Err(ResolveIntoError::Capacity);
        }
        self.bytes[self.length] = value;
        self.length += 1;
        Ok(())
    }

    fn truncate(&mut self, length: usize) {
        assert!(length <= self.length);
        self.length = length;
    }

    fn insert_str(&mut self, index: usize, value: &str) -> Result<(), ResolveIntoError> {
        assert!(index <= self.length);
        let end = self
            .length
            .checked_add(value.len())
            .ok_or(ResolveIntoError::Capacity)?;
        if end > N {
            return Err(ResolveIntoError::Capacity);
        }
        self.bytes
            .copy_within(index..self.length, index + value.len());
        self.bytes[index..index + value.len()].copy_from_slice(value.as_bytes());
        self.length = end;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_type::{ResolveFormHrefError, resolve_form_href};

    use super::super::{PROPERTY_FORMS, ROOT_PROPERTIES, Snapshot};

    fn selected_form_mut(thing: &mut crate::thing::Thing) -> &mut crate::form::Form {
        &mut thing
            .properties
            .as_mut()
            .unwrap()
            .get_mut("alpha")
            .unwrap()
            ._interaction
            .forms[0]
    }

    fn selected_snapshot_form(snapshot: &Snapshot) -> u32 {
        let properties = snapshot.child(snapshot.root, ROOT_PROPERTIES);
        let property = snapshot.map_get(properties, "alpha").unwrap();
        let forms = snapshot.child(property, PROPERTY_FORMS);
        snapshot.child(forms, 0)
    }

    fn configure_case(base: Option<&str>, href: &str) -> crate::thing::Thing {
        let mut thing = super::super::typed_corpus_shared::typed_corpus();
        thing.base = base.map(|base| BaseUri::parse(base).unwrap());
        selected_form_mut(&mut thing).href = FormHref::parse(href).unwrap();
        thing
    }

    fn assert_output_matches(
        result: Result<ResolveInto, ResolveIntoError>,
        output: &FixedUriBuffer<512>,
        raw: BorrowedHref<'_>,
        oracle: &Result<crate::data_type::ResolvedFormHref, ResolveFormHrefError>,
    ) {
        match (result, oracle) {
            (Ok(ResolveInto::AliasRaw), Ok(expected)) => {
                assert_eq!(raw.as_str(), expected.as_str());
                assert_eq!(
                    matches!(raw, BorrowedHref::Template(_)),
                    expected.is_template()
                );
                assert_eq!(output.as_str(), "");
            }
            (Ok(ResolveInto::Written), Ok(expected)) => {
                assert_eq!(output.as_str(), expected.as_str());
                assert!(!expected.is_template());
            }
            (Err(ResolveIntoError::TemplateBase), Err(ResolveFormHrefError::TemplateBase(_))) => {}
            (Err(ResolveIntoError::Resolution), Err(ResolveFormHrefError::Resolve(_))) => {}
            (actual, expected) => {
                panic!("shared result {actual:?} differs from oracle {expected:?}")
            }
        }
    }

    fn assert_case(base: Option<&str>, href: &str) {
        let thing = configure_case(base, href);
        let form = &thing
            .properties
            .as_ref()
            .unwrap()
            .get("alpha")
            .unwrap()
            ._interaction
            .forms[0];
        let oracle = resolve_form_href(thing.base.as_ref(), &form.href);

        let typed_base = thing.base.as_ref().map(BorrowedBase::from_typed);
        let typed_href = BorrowedHref::from_typed(&form.href);
        let mut typed_output = FixedUriBuffer::<512>::new();
        let typed_result = resolve_form_href_into(typed_base, typed_href, &mut typed_output);
        assert_output_matches(typed_result, &typed_output, typed_href, &oracle);

        let snapshot = Snapshot::normalize(&thing);
        assert_eq!(snapshot.base(), typed_base);
        let snapshot_form = selected_snapshot_form(&snapshot);
        let snapshot_href = snapshot.form_href(snapshot_form);
        assert_eq!(snapshot_href, typed_href);

        let mut snapshot_output = FixedUriBuffer::<512>::new();
        let snapshot_result =
            resolve_form_href_into(snapshot.base(), snapshot_href, &mut snapshot_output);
        assert_output_matches(snapshot_result, &snapshot_output, snapshot_href, &oracle);

        match (&oracle, snapshot.resolved_form_href(snapshot_form)) {
            (Ok(expected), Ok(actual)) => {
                assert_eq!(actual.as_str(), expected.as_str());
                assert_eq!(
                    matches!(actual, BorrowedHref::Template(_)),
                    expected.is_template()
                );
            }
            (Err(ResolveFormHrefError::TemplateBase(_)), Err(ResolveIntoError::TemplateBase)) => {}
            (Err(ResolveFormHrefError::Resolve(_)), Err(ResolveIntoError::Resolution)) => {}
            (expected, actual) => {
                panic!("cached result {actual:?} differs from oracle {expected:?}")
            }
        }
    }

    #[test]
    fn shared_uri_kernel_matches_current_td_for_required_resolution_shapes() {
        let cases = [
            (
                Some("https://base.example/a/b/?old=1"),
                "https://target.example/x/../y?mode=full#part",
            ),
            (
                Some("https://base.example/a/b/c/"),
                "../d/./e?mode=full#part",
            ),
            (None, "../d/./e?mode=full#part"),
            (
                Some("https://base.example/a/b/c/"),
                "properties/{name}?q={query}#part",
            ),
            (Some("https://base.example/{tenant}/"), "relative/path"),
            (
                Some("https://base.example/{tenant}/"),
                "https://target.example/absolute",
            ),
            (Some("https://base.example/a/b/c?old=1"), "?mode=full#part"),
            (Some("https://base.example/a/b/c?old=1"), "#part"),
            (Some("https://base.example/a/b/c/"), "/x/./y/../z?q=1#f"),
            (Some("https://base.example/a/b/c/"), "%2e%2e/d"),
            (Some("https://base.example/a/b/c/"), "/x/%2E%2E/y"),
            (
                Some("https://base.example/a/b/c/"),
                "//other.example/x?q=1#f",
            ),
            (Some("https://base.example/a/b/c/#base-fragment"), "child"),
            (Some("urn:example:opaque"), "child"),
            (Some("urn:example:opaque?old=1"), "#part"),
            (Some("foo:/a/b"), "/.//g"),
        ];

        for (base, href) in cases {
            assert_case(base, href);
        }
    }

    #[test]
    fn shared_uri_kernel_matches_rfc3986_reference_matrix() {
        let base = Some("http://a/b/c/d;p?q");
        for href in [
            "g:h",
            "g",
            "./g",
            "g/",
            "/g",
            "//g",
            "?y",
            "g?y",
            "#s",
            "g#s",
            "g?y#s",
            "",
            ".",
            "./",
            "..",
            "../",
            "../g",
            "../..",
            "../../",
            "../../g",
            "../../../g",
            "/./g",
            "/../g",
            "g.",
            ".g",
            "g..",
            "..g",
            "./../g",
            "./g/.",
            "g/./h",
            "g/../h",
            "g;x=1/./y",
            "g;x=1/../y",
            "g?y/./x",
            "g?y/../x",
            "g#s/./x",
            "g#s/../x",
        ] {
            assert_case(base, href);
        }
    }

    #[test]
    fn uri_preprocessing_and_borrowed_query_allocation_intervals_are_separate() {
        let thing = configure_case(
            Some("https://base.example/a/b/c/"),
            "../d/./e?mode=full#part",
        );
        let form = &thing
            .properties
            .as_ref()
            .unwrap()
            .get("alpha")
            .unwrap()
            ._interaction
            .forms[0];

        let (_, owned_query_allocations) =
            super::super::semantic_kernel_probe::count_allocations(|| {
                resolve_form_href(thing.base.as_ref(), &form.href).unwrap()
            });
        assert!(owned_query_allocations > 0);

        let ((shared_result, shared_text), shared_allocations) =
            super::super::semantic_kernel_probe::count_allocations(|| {
                let mut output = FixedUriBuffer::<512>::new();
                let result = resolve_form_href_into(
                    thing.base.as_ref().map(BorrowedBase::from_typed),
                    BorrowedHref::from_typed(&form.href),
                    &mut output,
                );
                (result, output.as_str().len())
            });
        assert_eq!(shared_result, Ok(ResolveInto::Written));
        assert!(shared_text > 0);
        assert_eq!(shared_allocations, 0);

        let (snapshot, preprocessing_allocations) =
            super::super::semantic_kernel_probe::count_allocations(|| Snapshot::normalize(&thing));
        assert_eq!(preprocessing_allocations, 6);

        let form = selected_snapshot_form(&snapshot);
        let (resolved, query_allocations) =
            super::super::semantic_kernel_probe::count_allocations(|| {
                snapshot.resolved_form_href(form).unwrap()
            });
        assert_eq!(
            resolved.as_str(),
            "https://base.example/a/b/d/e?mode=full#part"
        );
        assert_eq!(query_allocations, 0);
    }

    #[test]
    fn uri_cache_cost_uses_only_existing_retained_arenas() {
        let thing = super::super::typed_corpus_shared::typed_corpus();
        let snapshot = Snapshot::normalize(&thing);
        let cost = snapshot.uri_cache_cost;

        assert_eq!(cost.form_edges, 9);
        assert_eq!(cost.derived_nodes, 7);
        assert_eq!(cost.derived_bytes, 284);
        assert_eq!(
            cost.retained_requested_bytes(),
            cost.form_edges as usize * core::mem::size_of::<super::super::RetainedEdge>()
                + cost.derived_nodes as usize * core::mem::size_of::<super::super::RetainedNode>()
                + cost.derived_bytes as usize
        );
        assert_eq!(cost.retained_requested_bytes(), 496);
        assert_eq!(snapshot.arena.footprint().retained_allocation_count, 3);
    }
}
