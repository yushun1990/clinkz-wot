//! Non-production, complete literal JSON value-layer construction witness.
//! This is not Thing field decoding, a second Basic validator, or admission.
#![no_std]

extern crate alloc;

use clinkz_wot_foundation::{WorkBudget, WorkClass as W};
use core::{cmp::Ordering, mem};
use serde_json::Value;
use validated_thing_arena_layout_probe::{
    Error as ArenaError, Footprint, RetainedEdge as Edge, RetainedNode as Node,
    staged::{Site, Storage},
};
use validated_thing_strict_number_lexeme_probe::{Feed, NumberLexeme};

const LIVE: u32 = 1 << 31;
const DEAD: u32 = u32::MAX;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Null,
    False,
    True,
    String,
    Number,
    Array,
    Object,
    Entry,
}

fn container(kind: u32) -> bool {
    matches!(kind & !LIVE, x if x == Kind::Array as u32 || x == Kind::Object as u32 || x == Kind::Entry as u32)
}

/// Finite fixture controls, deliberately not the frozen admission projection.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub input_bytes: usize,
    pub nodes: usize,
    pub edges: usize,
    pub bytes: usize,
    pub frames: usize,
    pub number: usize,
    pub lifetime: u64,
    pub source: u64,
    pub temporary: u64,
    pub peak: u64,
    pub contiguous: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 1_000_000,
            nodes: 100_000,
            edges: 100_000,
            bytes: 1_000_000,
            frames: 4096,
            number: 256,
            lifetime: 100_000_000,
            source: 8_000_000,
            temporary: 16_000_000,
            peak: 24_000_000,
            contiguous: 8_000_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Input,
    Sort,
    Duplicates,
    Reachability,
    Compact,
    Seal,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Syntax,
    InputBytes,
    Nodes,
    Edges,
    Bytes,
    Frames,
    RawNumber,
    DecodedNumber,
    Lifetime,
    Memory,
    Arithmetic,
    Allocation,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Failure {
    pub cause: Cause,
    pub phase: Phase,
    pub offset: usize,
    pub live_after_rollback: u64,
    pub allocations: u64,
    pub releases: u64,
    pub trace: Trace,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Trace {
    /// Accepted class debits: DocumentNodes, input, output, prepaid cleanup.
    pub work: [u64; 4],
    pub wire_observed: usize,
    pub grow_copies: usize,
    pub seal_copies: usize,
    pub key_bytes: usize,
    pub discarded_nodes: usize,
}

enum Input<'a> {
    Wire(&'a [u8]),
    Typed(&'a Value),
}
enum Iter<'a> {
    Array(core::slice::Iter<'a, Value>),
    Object(serde_json::map::Iter<'a>),
}

// Only borrowed public iterators/scalars. No recursive destructor or owning
// String/Value/task tree resides in the one frame arena.
struct Frame<'a> {
    node: u32,
    kind: Kind,
    mode: u8,
    entry: u32,
    value: Option<&'a Value>,
    iter: Option<Iter<'a>>,
    next: usize,
    end: usize,
}
const _: () = assert!(!mem::needs_drop::<Frame<'static>>());
const _: () = assert!(!mem::needs_drop::<Node>() && !mem::needs_drop::<Edge>());

#[derive(Clone, Copy)]
enum Role {
    Value,
    Key,
}

#[derive(Clone, Copy)]
enum StringPhase {
    Normal,
    Escape,
    Hex,
    HighSlash,
    HighU,
    LowHex,
    Utf8,
}

#[derive(Clone, Copy)]
struct StringState {
    node: u32,
    role: Role,
    phase: StringPhase,
    hex: u16,
    digits: u8,
    high: u16,
    buffer: [u8; 4],
    length: u8,
    expected: u8,
    output: u8,
    pending: u8,
}

struct NumberState {
    node: u32,
    lexer: NumberLexeme,
    start: usize,
    length: usize,
    exponent: Option<usize>,
    sign: bool,
    negative_zero: bool,
    first: Option<u8>,
}

#[derive(Clone, Copy)]
struct KeyCompare {
    left: Node,
    right: Node,
    index: usize,
    byte: Option<u8>,
}

#[derive(Clone, Copy)]
struct Sort {
    index: usize,
    position: usize,
    pivot: Option<Edge>,
    mode: u8, // compare, move, place
    compare: Option<KeyCompare>,
}

#[derive(Clone, Copy)]
struct Duplicates {
    node: usize,
    position: usize,
    previous: Option<usize>,
    compare: Option<KeyCompare>,
    discard: Option<usize>,
}

enum State<'a> {
    Init,
    Wire,
    EndWire,
    Typed(&'a Value),
    TypedNext,
    NewContainer(Kind, Option<Iter<'a>>),
    PushContainer(u32, Kind, Option<Iter<'a>>),
    NewString(Role, Option<&'a str>),
    String(StringState),
    TypedText {
        node: u32,
        text: &'a str,
        position: usize,
        role: Role,
    },
    NewNumber(u8, usize),
    Number(NumberState),
    NumberOutput {
        node: u32,
        start: usize,
        end: usize,
        position: usize,
        exponent: Option<usize>,
        insert: bool,
    },
    Scalar(Kind),
    Literal {
        node: u32,
        expected: &'static [u8],
        position: usize,
    },
    Done(u32, Role),
    KeyEntry(u32),
    KeyEdges {
        key: u32,
        stage: u8,
    },
    Sort(Sort),
    Ranges(usize),
    Duplicates(Duplicates),
    Mark(Option<u32>),
    EdgeCompact {
        node: usize,
        position: usize,
        end: usize,
        start: usize,
        output: usize,
    },
    ByteCompact {
        node: usize,
        position: usize,
        end: usize,
        start: usize,
        output: usize,
        active: bool,
    },
    NodeCompact {
        input: usize,
        output: usize,
    },
    Seal(usize),
    Finished,
    Busy,
}

pub struct Cursor<'a> {
    input: Input<'a>,
    limits: Limits,
    storage: Storage<Frame<'a>>,
    state: State<'a>,
    position: usize,
    cached: Option<u8>,
    lifetime: u64,
    live_nodes: u32,
    root: Option<u32>,
    phase: Phase,
    trace: Trace,
}

// Boxing a continuation would add an unauthorized owning allocation site.
#[allow(clippy::large_enum_variant)]
pub enum Progress<'a> {
    Pending(Cursor<'a>),
    Complete(OwnedValue),
    Failed(Failure),
}

impl<'a> Cursor<'a> {
    pub fn from_json(input: &'a [u8], limits: Limits) -> Self {
        Self::new(Input::Wire(input), limits)
    }
    pub fn from_value(value: &'a Value, limits: Limits) -> Self {
        Self::new(Input::Typed(value), limits)
    }
    fn new(input: Input<'a>, limits: Limits) -> Self {
        Self {
            input,
            storage: Storage::new(
                limits.source,
                limits.temporary,
                limits.peak,
                limits.contiguous,
            ),
            state: State::Init,
            position: 0,
            cached: None,
            lifetime: limits.lifetime,
            limits,
            root: None,
            live_nodes: 0,
            phase: Phase::Input,
            trace: Trace::default(),
        }
    }
    pub fn trace(&self) -> Trace {
        self.trace
    }
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn lifetime_remaining(&self) -> u64 {
        self.lifetime
    }
    pub fn live_bytes(&self) -> u64 {
        self.storage.live_bytes()
    }
    pub fn footprint(&self) -> Footprint {
        self.storage.footprint()
    }

    pub fn step(mut self, budget: &mut WorkBudget, cancel: bool) -> Progress<'a> {
        if cancel {
            return self.fail(Cause::Cancelled);
        }
        loop {
            if matches!(self.state, State::Finished) {
                let value = OwnedValue {
                    storage: self.storage.into_owned(),
                    trace: self.trace,
                };
                return Progress::Complete(value);
            }
            match self.tick(budget) {
                Ok(true) => {}
                Ok(false) => return Progress::Pending(self),
                Err(cause) => return self.fail(cause),
            }
        }
    }

    fn fail(mut self, cause: Cause) -> Progress<'a> {
        self.storage.clear();
        Progress::Failed(Failure {
            cause,
            phase: self.phase,
            offset: self.position,
            live_after_rollback: self.storage.live_bytes(),
            allocations: self.storage.allocations(),
            releases: self.storage.releases(),
            trace: self.trace,
        })
    }

    fn pay(&mut self, budget: &mut WorkBudget, classes: &[W]) -> Result<bool, Cause> {
        if classes.iter().any(|&class| budget.remaining(class) == 0) {
            return Ok(false);
        }
        if self.lifetime < classes.len() as u64 {
            return Err(Cause::Lifetime);
        }
        for &class in classes {
            budget.consume(class, 1).unwrap();
            self.trace.work[match class {
                W::DocumentNodes => 0,
                W::CodecInputBytes => 1,
                W::CodecOutputBytes => 2,
                W::CleanupItems => 3,
                _ => unreachable!(),
            }] += 1;
        }
        self.lifetime -= classes.len() as u64;
        Ok(true)
    }

    fn ensure(&mut self, site: Site, extra: usize, budget: &mut WorkBudget) -> Result<bool, Cause> {
        let length = self
            .storage
            .len(site)
            .checked_add(extra)
            .ok_or(Cause::Arithmetic)?;
        let (limit, cause) = match site {
            Site::Nodes => (self.limits.nodes.min((LIVE - 1) as usize), Cause::Nodes),
            Site::Edges => (self.limits.edges.min((LIVE - 1) as usize), Cause::Edges),
            Site::Bytes => (self.limits.bytes.min(u32::MAX as usize), Cause::Bytes),
            Site::Frames => (self.limits.frames, Cause::Frames),
        };
        if length > limit {
            return Err(cause);
        }
        if length <= self.storage.capacity(site) {
            return Ok(true);
        }
        let capacity = self
            .storage
            .capacity(site)
            .max(1)
            .checked_mul(2)
            .ok_or(Cause::Arithmetic)?
            .max(length);
        if !self.pay(budget, &[W::CleanupItems])? {
            return Ok(false);
        }
        self.storage
            .begin_grow(site, capacity)
            .map_err(arena_cause)?;
        Ok(false)
    }

    fn read(&mut self, budget: &mut WorkBudget) -> Result<Option<Option<u8>>, Cause> {
        if let Some(byte) = self.cached.take() {
            return Ok(Some(Some(byte)));
        }
        let Input::Wire(input) = self.input else {
            unreachable!()
        };
        if self.position == input.len() {
            return Ok(Some(None));
        }
        if !self.pay(budget, &[W::CodecInputBytes])? {
            return Ok(None);
        }
        let byte = input[self.position];
        self.position += 1;
        self.trace.wire_observed += 1;
        Ok(Some(Some(byte)))
    }

    fn node(&mut self, kind: Kind) -> u32 {
        let id = self.storage.len(Site::Nodes) as u32;
        self.storage.push_node(Node {
            kind: kind as u32,
            first_edge: 0,
            edge_count: 0,
            first_byte: self.storage.len(Site::Bytes) as u32,
            byte_count: 0,
        });
        id
    }
    fn edge(&mut self, parent: u32, target: u32) {
        self.storage.push_edge(Edge {
            target,
            original_index: parent,
        });
    }
    fn finish_text(&mut self, node: u32) {
        let mut record = self.storage.node(node as usize);
        record.byte_count = self.storage.len(Site::Bytes) as u32 - record.first_byte;
        self.storage.set_node(node as usize, record);
    }
    fn sort_start(&mut self) -> State<'a> {
        self.phase = Phase::Sort;
        State::Sort(Sort {
            index: 1,
            position: 1,
            pivot: None,
            mode: 0,
            compare: None,
        })
    }

    fn key(&self, edge: Edge) -> Node {
        let entry = self.storage.node(edge.target as usize);
        self.storage.node(entry.first_byte as usize)
    }
    fn compare(
        &mut self,
        compare: &mut KeyCompare,
        budget: &mut WorkBudget,
    ) -> Result<Option<Ordering>, Cause> {
        let n = compare.index;
        if n == compare.left.byte_count as usize || n == compare.right.byte_count as usize {
            return Ok(Some(compare.left.byte_count.cmp(&compare.right.byte_count)));
        }
        if !self.pay(budget, &[W::CodecInputBytes])? {
            return Ok(None);
        }
        self.trace.key_bytes += 1;
        if let Some(left) = compare.byte.take() {
            let right = self.storage.byte(compare.right.first_byte as usize + n);
            if left != right {
                return Ok(Some(left.cmp(&right)));
            }
            compare.index += 1;
        } else {
            compare.byte = Some(self.storage.byte(compare.left.first_byte as usize + n));
        }
        Ok(None)
    }

    fn tick(&mut self, budget: &mut WorkBudget) -> Result<bool, Cause> {
        if !self.pay(budget, &[W::DocumentNodes])? {
            return Ok(false);
        }
        if let Some(site) = self.storage.transferring() {
            let copying = self.storage.copy_pending();
            if copying
                && site == Site::Bytes
                && !self.pay(budget, &[W::CodecInputBytes, W::CodecOutputBytes])?
            {
                return Ok(false);
            }
            self.storage.copy_one();
            if copying && self.phase == Phase::Seal {
                self.trace.seal_copies += 1;
            } else if copying {
                self.trace.grow_copies += 1;
            }
            return Ok(true);
        }
        let state = mem::replace(&mut self.state, State::Busy);
        let (next, advanced) = match state {
            State::Init => match self.input {
                Input::Wire(input) => {
                    if input.len() > self.limits.input_bytes {
                        return Err(Cause::InputBytes);
                    }
                    (State::Wire, true)
                }
                Input::Typed(value) => (State::Typed(value), true),
            },
            State::Wire => self.wire(budget)?,
            State::EndWire => match self.read(budget)? {
                None => (State::EndWire, false),
                Some(None) => (self.sort_start(), true),
                Some(Some(byte)) if whitespace(byte) => (State::EndWire, true),
                _ => return Err(Cause::Syntax),
            },
            State::Typed(value) => match value {
                Value::Null => (State::Scalar(Kind::Null), true),
                Value::Bool(value) => (
                    State::Scalar(if *value { Kind::True } else { Kind::False }),
                    true,
                ),
                Value::String(text) => (State::NewString(Role::Value, Some(text)), true),
                Value::Number(number) => {
                    let text = number.as_str();
                    if text.len() > self.limits.number {
                        return Err(Cause::DecodedNumber);
                    }
                    if !self.ensure(Site::Nodes, 1, budget)? {
                        (State::Typed(value), false)
                    } else {
                        let node = self.node(Kind::Number);
                        (
                            State::TypedText {
                                node,
                                text,
                                position: 0,
                                role: Role::Value,
                            },
                            true,
                        )
                    }
                }
                Value::Array(values) => (
                    State::NewContainer(Kind::Array, Some(Iter::Array(values.iter()))),
                    true,
                ),
                Value::Object(values) => (
                    State::NewContainer(Kind::Object, Some(Iter::Object(values.iter()))),
                    true,
                ),
            },
            State::TypedNext => {
                let frame = self.storage.frame_mut();
                match frame.iter.as_mut().unwrap() {
                    Iter::Array(iter) => match iter.next() {
                        Some(value) => (State::Typed(value), true),
                        None => {
                            let id = self.storage.pop_frame().node;
                            (State::Done(id, Role::Value), true)
                        }
                    },
                    Iter::Object(iter) => match iter.next() {
                        Some((key, value)) => {
                            frame.value = Some(value);
                            (State::NewString(Role::Key, Some(key)), true)
                        }
                        None => {
                            let id = self.storage.pop_frame().node;
                            (State::Done(id, Role::Value), true)
                        }
                    },
                }
            }
            State::NewContainer(kind, iter) => {
                if !self.ensure(Site::Nodes, 1, budget)? {
                    (State::NewContainer(kind, iter), false)
                } else {
                    let node = self.node(kind);
                    (State::PushContainer(node, kind, iter), true)
                }
            }
            State::PushContainer(node, kind, iter) => {
                if !self.ensure(Site::Frames, 1, budget)? {
                    (State::PushContainer(node, kind, iter), false)
                } else {
                    let typed = iter.is_some();
                    self.storage.push_frame(Frame {
                        node,
                        kind,
                        mode: 0,
                        entry: 0,
                        value: None,
                        iter,
                        next: 0,
                        end: 0,
                    });
                    (if typed { State::TypedNext } else { State::Wire }, true)
                }
            }
            State::NewString(role, text) => {
                if !self.ensure(Site::Nodes, 1, budget)? {
                    (State::NewString(role, text), false)
                } else {
                    let node = self.node(Kind::String);
                    (
                        match text {
                            Some(text) => State::TypedText {
                                node,
                                text,
                                position: 0,
                                role,
                            },
                            None => State::String(StringState {
                                node,
                                role,
                                phase: StringPhase::Normal,
                                hex: 0,
                                digits: 0,
                                high: 0,
                                buffer: [0; 4],
                                length: 0,
                                expected: 0,
                                output: 0,
                                pending: 0,
                            }),
                        },
                        true,
                    )
                }
            }
            State::TypedText {
                node,
                text,
                position,
                role,
            } => {
                if position == text.len() {
                    self.finish_text(node);
                    (State::Done(node, role), true)
                } else if !self.ensure(Site::Bytes, 1, budget)?
                    || !self.pay(budget, &[W::CodecInputBytes, W::CodecOutputBytes])?
                {
                    (
                        State::TypedText {
                            node,
                            text,
                            position,
                            role,
                        },
                        false,
                    )
                } else {
                    self.storage.push_byte(text.as_bytes()[position]);
                    (
                        State::TypedText {
                            node,
                            text,
                            position: position + 1,
                            role,
                        },
                        true,
                    )
                }
            }
            State::String(string) => self.string(string, budget)?,
            State::NewNumber(first, start) => {
                if !self.ensure(Site::Nodes, 1, budget)? {
                    (State::NewNumber(first, start), false)
                } else {
                    let node = self.node(Kind::Number);
                    (
                        State::Number(NumberState {
                            node,
                            lexer: NumberLexeme::new(self.limits.number),
                            start,
                            length: 0,
                            exponent: None,
                            sign: false,
                            negative_zero: first == b'-',
                            first: Some(first),
                        }),
                        true,
                    )
                }
            }
            State::Number(mut number) => {
                let byte = if let Some(first) = number.first.take() {
                    Some(Some(first))
                } else {
                    self.read(budget)?
                };
                match byte {
                    None => (State::Number(number), false),
                    Some(byte) => {
                        let feed = match byte {
                            Some(byte) => number.lexer.feed(byte),
                            None => number.lexer.finish_eof(),
                        };
                        match feed {
                            Feed::Consumed => {
                                let byte = byte.unwrap();
                                if matches!(byte, b'e' | b'E') {
                                    number.exponent = Some(number.length);
                                } else if number.exponent.is_some()
                                    && number.exponent == number.length.checked_sub(1)
                                    && matches!(byte, b'+' | b'-')
                                {
                                    number.sign = true;
                                }
                                if number.length == 1 {
                                    number.negative_zero &= byte == b'0';
                                }
                                number.length += 1;
                                (State::Number(number), true)
                            }
                            Feed::Complete => {
                                self.cached = byte;
                                let negative_zero = number.length == 2 && number.negative_zero;
                                let insert = number.exponent.is_some() && !number.sign;
                                let length = number
                                    .length
                                    .checked_add(usize::from(insert))
                                    .ok_or(Cause::Arithmetic)?
                                    - usize::from(negative_zero);
                                if length > self.limits.number {
                                    return Err(Cause::DecodedNumber);
                                }
                                (
                                    State::NumberOutput {
                                        node: number.node,
                                        start: number.start,
                                        end: number.start + number.length,
                                        position: number.start + usize::from(negative_zero),
                                        exponent: number.exponent.map(|n| n + number.start),
                                        insert,
                                    },
                                    true,
                                )
                            }
                            Feed::Limit => return Err(Cause::RawNumber),
                            Feed::Invalid => return Err(Cause::Syntax),
                        }
                    }
                }
            }
            State::NumberOutput {
                node,
                start,
                end,
                position,
                exponent,
                insert,
            } => {
                if position == end {
                    self.finish_text(node);
                    (State::Done(node, Role::Value), true)
                } else if !self.ensure(Site::Bytes, 1, budget)? {
                    (
                        State::NumberOutput {
                            node,
                            start,
                            end,
                            position,
                            exponent,
                            insert,
                        },
                        false,
                    )
                } else if insert && exponent == position.checked_sub(1) {
                    if !self.pay(budget, &[W::CodecOutputBytes])? {
                        (
                            State::NumberOutput {
                                node,
                                start,
                                end,
                                position,
                                exponent,
                                insert,
                            },
                            false,
                        )
                    } else {
                        self.storage.push_byte(b'+');
                        (
                            State::NumberOutput {
                                node,
                                start,
                                end,
                                position,
                                exponent,
                                insert: false,
                            },
                            true,
                        )
                    }
                } else if !self.pay(budget, &[W::CodecInputBytes, W::CodecOutputBytes])? {
                    (
                        State::NumberOutput {
                            node,
                            start,
                            end,
                            position,
                            exponent,
                            insert,
                        },
                        false,
                    )
                } else {
                    let Input::Wire(input) = self.input else {
                        unreachable!()
                    };
                    self.storage.push_byte(if exponent == Some(position) {
                        b'e'
                    } else {
                        input[position]
                    });
                    (
                        State::NumberOutput {
                            node,
                            start,
                            end,
                            position: position + 1,
                            exponent,
                            insert,
                        },
                        true,
                    )
                }
            }
            State::Scalar(kind) => {
                if !self.ensure(Site::Nodes, 1, budget)? {
                    (State::Scalar(kind), false)
                } else {
                    let node = self.node(kind);
                    (
                        if matches!(self.input, Input::Wire(_)) {
                            State::Literal {
                                node,
                                expected: match kind {
                                    Kind::Null => b"null",
                                    Kind::True => b"true",
                                    Kind::False => b"false",
                                    _ => unreachable!(),
                                },
                                position: 1,
                            }
                        } else {
                            State::Done(node, Role::Value)
                        },
                        true,
                    )
                }
            }
            State::Literal {
                node,
                expected,
                position,
            } => {
                if position == expected.len() {
                    (State::Done(node, Role::Value), true)
                } else {
                    match self.read(budget)? {
                        None => (
                            State::Literal {
                                node,
                                expected,
                                position,
                            },
                            false,
                        ),
                        Some(Some(byte)) if byte == expected[position] => (
                            State::Literal {
                                node,
                                expected,
                                position: position + 1,
                            },
                            true,
                        ),
                        _ => return Err(Cause::Syntax),
                    }
                }
            }
            State::Done(node, Role::Key) => (State::KeyEntry(node), true),
            State::KeyEntry(key) => {
                if !self.ensure(Site::Nodes, 1, budget)? {
                    (State::KeyEntry(key), false)
                } else {
                    let entry = self.node(Kind::Entry);
                    let mut record = self.storage.node(entry as usize);
                    record.first_byte = key;
                    self.storage.set_node(entry as usize, record);
                    self.storage.frame_mut().entry = entry;
                    (State::KeyEdges { key, stage: 0 }, true)
                }
            }
            State::KeyEdges { key, stage } => {
                if !self.ensure(Site::Edges, 1, budget)? {
                    (State::KeyEdges { key, stage }, false)
                } else {
                    let frame = self.storage.frame_mut();
                    let (parent, child) = if stage == 0 {
                        (frame.node, frame.entry)
                    } else {
                        (frame.entry, key)
                    };
                    self.edge(parent, child);
                    if stage == 0 {
                        (State::KeyEdges { key, stage: 1 }, true)
                    } else {
                        let frame = self.storage.frame_mut();
                        frame.mode = 2; // colon for wire, value for typed
                        (
                            if let Some(value) = frame.value.take() {
                                State::Typed(value)
                            } else {
                                State::Wire
                            },
                            true,
                        )
                    }
                }
            }
            State::Done(node, Role::Value) => {
                if self.storage.len(Site::Frames) == 0 {
                    self.root = Some(node);
                    (
                        if matches!(self.input, Input::Wire(_)) {
                            State::EndWire
                        } else {
                            self.sort_start()
                        },
                        true,
                    )
                } else if !self.ensure(Site::Edges, 1, budget)? {
                    (State::Done(node, Role::Value), false)
                } else {
                    let frame = self.storage.frame_mut();
                    let parent = if frame.kind == Kind::Object {
                        frame.entry
                    } else {
                        frame.node
                    };
                    frame.mode = 4;
                    let typed = frame.iter.is_some();
                    self.edge(parent, node);
                    (if typed { State::TypedNext } else { State::Wire }, true)
                }
            }
            State::Sort(sort) => self.sort(sort, budget)?,
            State::Ranges(index) => {
                if index == self.storage.len(Site::Edges) {
                    self.phase = Phase::Duplicates;
                    (
                        State::Duplicates(Duplicates {
                            node: 0,
                            position: 0,
                            previous: None,
                            compare: None,
                            discard: None,
                        }),
                        true,
                    )
                } else {
                    let edge = self.storage.edge(index);
                    let mut node = self.storage.node(edge.original_index as usize);
                    if node.edge_count == 0 {
                        node.first_edge = index as u32;
                    }
                    node.edge_count += 1;
                    self.storage.set_node(edge.original_index as usize, node);
                    (State::Ranges(index + 1), true)
                }
            }
            State::Duplicates(duplicates) => self.duplicates(duplicates, budget)?,
            State::Mark(candidate) => {
                if let Some(id) = candidate {
                    if !self.ensure(Site::Frames, 1, budget)? {
                        (State::Mark(candidate), false)
                    } else {
                        let mut node = self.storage.node(id as usize);
                        let next = node.first_edge as usize;
                        let end = next + node.edge_count as usize;
                        let is_container = container(node.kind);
                        if is_container {
                            node.first_byte = self.live_nodes;
                        } else {
                            node.edge_count = self.live_nodes;
                        }
                        node.kind |= LIVE;
                        self.live_nodes =
                            self.live_nodes.checked_add(1).ok_or(Cause::Arithmetic)?;
                        self.storage.set_node(id as usize, node);
                        self.storage.push_frame(Frame {
                            node: id,
                            kind: Kind::Entry,
                            mode: 0,
                            entry: 0,
                            value: None,
                            iter: None,
                            next,
                            end,
                        });
                        (State::Mark(None), true)
                    }
                } else if self.storage.len(Site::Frames) == 0 {
                    // Assign final IDs in physical node order, independently
                    // of the reachability traversal's sorted member order.
                    self.phase = Phase::Compact;
                    self.live_nodes = 0;
                    (
                        State::NodeCompact {
                            input: 0,
                            output: usize::MAX,
                        },
                        true,
                    )
                } else {
                    let frame = self.storage.frame_mut();
                    if frame.next == frame.end {
                        self.storage.pop_frame();
                        (State::Mark(None), true)
                    } else {
                        let index = frame.next;
                        frame.next += 1;
                        let mut edge = self.storage.edge(index);
                        if edge.original_index == DEAD {
                            (State::Mark(None), true)
                        } else {
                            edge.original_index |= LIVE;
                            self.storage.set_edge(index, edge);
                            (State::Mark(Some(edge.target)), true)
                        }
                    }
                }
            }
            State::NodeCompact {
                input,
                output: usize::MAX,
            } => {
                if input == self.storage.len(Site::Nodes) {
                    (
                        State::EdgeCompact {
                            node: 0,
                            position: 0,
                            end: 0,
                            start: 0,
                            output: 0,
                        },
                        true,
                    )
                } else {
                    let mut node = self.storage.node(input);
                    if node.kind & LIVE != 0 {
                        if container(node.kind) {
                            node.first_byte = self.live_nodes;
                        } else {
                            node.edge_count = self.live_nodes;
                        }
                        self.storage.set_node(input, node);
                        self.live_nodes += 1;
                    }
                    (
                        State::NodeCompact {
                            input: input + 1,
                            output: usize::MAX,
                        },
                        true,
                    )
                }
            }
            State::EdgeCompact {
                node,
                position,
                end,
                start,
                output,
            } => {
                if node == self.storage.len(Site::Nodes) {
                    self.storage.truncate(Site::Edges, output);
                    (
                        State::ByteCompact {
                            node: 0,
                            position: 0,
                            end: 0,
                            start: 0,
                            output: 0,
                            active: false,
                        },
                        true,
                    )
                } else if position < end {
                    let edge = self.storage.edge(position);
                    let mut new_output = output;
                    if edge.original_index != DEAD && edge.original_index & LIVE != 0 {
                        let target = self.storage.node(edge.target as usize);
                        let id = if container(target.kind) {
                            target.first_byte
                        } else {
                            target.edge_count
                        };
                        self.storage.set_edge(
                            output,
                            Edge {
                                target: id,
                                original_index: (output - start) as u32,
                            },
                        );
                        new_output += 1;
                    }
                    (
                        State::EdgeCompact {
                            node,
                            position: position + 1,
                            end,
                            start,
                            output: new_output,
                        },
                        true,
                    )
                } else {
                    let mut record = self.storage.node(node);
                    if record.kind & LIVE != 0 && container(record.kind) {
                        if end == 0 {
                            let first = record.first_edge as usize;
                            let count = record.edge_count as usize;
                            if count != 0 {
                                return self.store_next(State::EdgeCompact {
                                    node,
                                    position: first,
                                    end: first + count,
                                    start: output,
                                    output,
                                });
                            }
                        }
                        record.first_edge = start as u32;
                        record.edge_count = (output - start) as u32;
                        self.storage.set_node(node, record);
                    }
                    (
                        State::EdgeCompact {
                            node: node + 1,
                            position: 0,
                            end: 0,
                            start: output,
                            output,
                        },
                        true,
                    )
                }
            }
            State::ByteCompact {
                node,
                position,
                end,
                start,
                output,
                active,
            } => {
                if node == self.storage.len(Site::Nodes) {
                    self.storage.truncate(Site::Bytes, output);
                    (
                        State::NodeCompact {
                            input: 0,
                            output: 0,
                        },
                        true,
                    )
                } else if active && position < end {
                    if !self.pay(budget, &[W::CodecInputBytes, W::CodecOutputBytes])? {
                        (
                            State::ByteCompact {
                                node,
                                position,
                                end,
                                start,
                                output,
                                active,
                            },
                            false,
                        )
                    } else {
                        let byte = self.storage.byte(position);
                        self.storage.set_byte(output, byte);
                        (
                            State::ByteCompact {
                                node,
                                position: position + 1,
                                end,
                                start,
                                output: output + 1,
                                active,
                            },
                            true,
                        )
                    }
                } else {
                    let mut record = self.storage.node(node);
                    if active {
                        record.first_byte = start as u32;
                        self.storage.set_node(node, record);
                    } else if record.kind & LIVE != 0
                        && matches!(record.kind & !LIVE, x if x == Kind::String as u32 || x == Kind::Number as u32)
                    {
                        return self.store_next(State::ByteCompact {
                            node,
                            position: record.first_byte as usize,
                            end: record.first_byte as usize + record.byte_count as usize,
                            start: output,
                            output,
                            active: true,
                        });
                    }
                    (
                        State::ByteCompact {
                            node: node + 1,
                            position: 0,
                            end: 0,
                            start: output,
                            output,
                            active: false,
                        },
                        true,
                    )
                }
            }
            State::NodeCompact { input, output } => {
                if input == self.storage.len(Site::Nodes) {
                    self.storage.truncate(Site::Nodes, output);
                    self.storage.release_site(Site::Frames);
                    self.phase = Phase::Seal;
                    (State::Seal(0), true)
                } else {
                    let mut node = self.storage.node(input);
                    let live = node.kind & LIVE != 0;
                    if live {
                        node.kind &= !LIVE;
                        if container(node.kind) {
                            node.first_byte = 0;
                            node.byte_count = 0;
                        } else {
                            node.first_edge = 0;
                            node.edge_count = 0;
                        }
                        self.storage.set_node(output, node);
                    } else {
                        self.trace.discarded_nodes += 1;
                    }
                    (
                        State::NodeCompact {
                            input: input + 1,
                            output: output + usize::from(live),
                        },
                        true,
                    )
                }
            }
            State::Seal(index) => {
                if index == 3 {
                    self.storage.finish();
                    self.phase = Phase::Complete;
                    (State::Finished, true)
                } else {
                    let site = [Site::Nodes, Site::Edges, Site::Bytes][index];
                    if self.storage.len(site) != 0 && !self.pay(budget, &[W::CleanupItems])? {
                        (State::Seal(index), false)
                    } else {
                        self.storage.begin_seal(site).map_err(arena_cause)?;
                        (State::Seal(index + 1), true)
                    }
                }
            }
            State::Finished | State::Busy => unreachable!(),
        };
        self.state = next;
        Ok(advanced)
    }

    fn store_next(&mut self, state: State<'a>) -> Result<bool, Cause> {
        self.state = state;
        Ok(true)
    }

    fn wire(&mut self, budget: &mut WorkBudget) -> Result<(State<'a>, bool), Cause> {
        let Some(byte) = self.read(budget)? else {
            return Ok((State::Wire, false));
        };
        let byte = byte.ok_or(Cause::Syntax)?;
        if whitespace(byte) {
            return Ok((State::Wire, true));
        }
        if self.storage.len(Site::Frames) == 0 {
            // The strict TD entry requires one complete object.
            if byte != b'{' {
                return Err(Cause::Syntax);
            }
            return Ok((State::NewContainer(Kind::Object, None), true));
        }
        let frame = self.storage.frame_mut();
        if frame.kind == Kind::Object && frame.mode <= 1 {
            if byte == b'}' && frame.mode == 0 {
                let node = self.storage.pop_frame().node;
                return Ok((State::Done(node, Role::Value), true));
            }
            if byte != b'"' {
                return Err(Cause::Syntax);
            }
            return Ok((State::NewString(Role::Key, None), true));
        }
        if frame.kind == Kind::Object && frame.mode == 2 {
            if byte != b':' {
                return Err(Cause::Syntax);
            }
            frame.mode = 3;
            return Ok((State::Wire, true));
        }
        if frame.mode == 4 {
            let close = if frame.kind == Kind::Object {
                b'}'
            } else {
                b']'
            };
            if byte == close {
                let node = self.storage.pop_frame().node;
                return Ok((State::Done(node, Role::Value), true));
            }
            if byte != b',' {
                return Err(Cause::Syntax);
            }
            frame.mode = 1;
            return Ok((State::Wire, true));
        }
        if byte == b']' && frame.kind == Kind::Array && frame.mode == 0 {
            let node = self.storage.pop_frame().node;
            return Ok((State::Done(node, Role::Value), true));
        }
        Ok((
            match byte {
                b'{' => State::NewContainer(Kind::Object, None),
                b'[' => State::NewContainer(Kind::Array, None),
                b'"' => State::NewString(Role::Value, None),
                b'n' => State::Scalar(Kind::Null),
                b't' => State::Scalar(Kind::True),
                b'f' => State::Scalar(Kind::False),
                b'-' | b'0'..=b'9' => State::NewNumber(byte, self.position - 1),
                _ => return Err(Cause::Syntax),
            },
            true,
        ))
    }

    fn string(
        &mut self,
        mut string: StringState,
        budget: &mut WorkBudget,
    ) -> Result<(State<'a>, bool), Cause> {
        if string.output < string.pending {
            if !self.ensure(Site::Bytes, 1, budget)? || !self.pay(budget, &[W::CodecOutputBytes])? {
                return Ok((State::String(string), false));
            }
            self.storage
                .push_byte(string.buffer[string.output as usize]);
            string.output += 1;
            return Ok((State::String(string), true));
        }
        string.pending = 0;
        string.output = 0;
        let Some(byte) = self.read(budget)? else {
            return Ok((State::String(string), false));
        };
        let byte = byte.ok_or(Cause::Syntax)?;
        match string.phase {
            StringPhase::Normal => match byte {
                b'"' => {
                    self.finish_text(string.node);
                    return Ok((State::Done(string.node, string.role), true));
                }
                b'\\' => string.phase = StringPhase::Escape,
                0..=31 => return Err(Cause::Syntax),
                32..=127 => {
                    string.buffer[0] = byte;
                    string.pending = 1;
                }
                _ => {
                    string.expected = match byte {
                        0xc2..=0xdf => 2,
                        0xe0..=0xef => 3,
                        0xf0..=0xf4 => 4,
                        _ => return Err(Cause::Syntax),
                    };
                    string.buffer[0] = byte;
                    string.length = 1;
                    string.phase = StringPhase::Utf8;
                }
            },
            StringPhase::Escape => {
                let escaped = match byte {
                    b'"' | b'\\' | b'/' => Some(byte),
                    b'b' => Some(8),
                    b'f' => Some(12),
                    b'n' => Some(b'\n'),
                    b'r' => Some(b'\r'),
                    b't' => Some(b'\t'),
                    b'u' => None,
                    _ => return Err(Cause::Syntax),
                };
                if let Some(byte) = escaped {
                    string.buffer[0] = byte;
                    string.pending = 1;
                    string.phase = StringPhase::Normal;
                } else {
                    string.hex = 0;
                    string.digits = 0;
                    string.phase = StringPhase::Hex;
                }
            }
            StringPhase::Hex | StringPhase::LowHex => {
                let digit = (byte as char).to_digit(16).ok_or(Cause::Syntax)? as u16;
                string.hex = (string.hex << 4) | digit;
                string.digits += 1;
                if string.digits == 4 {
                    if matches!(string.phase, StringPhase::Hex)
                        && (0xd800..=0xdbff).contains(&string.hex)
                    {
                        string.high = string.hex;
                        string.phase = StringPhase::HighSlash;
                    } else {
                        let value = if matches!(string.phase, StringPhase::LowHex) {
                            if !(0xdc00..=0xdfff).contains(&string.hex) {
                                return Err(Cause::Syntax);
                            }
                            0x10000
                                + (((string.high - 0xd800) as u32) << 10)
                                + (string.hex - 0xdc00) as u32
                        } else {
                            string.hex as u32
                        };
                        let character = char::from_u32(value).ok_or(Cause::Syntax)?;
                        string.pending = character.encode_utf8(&mut string.buffer).len() as u8;
                        string.phase = StringPhase::Normal;
                    }
                }
            }
            StringPhase::HighSlash => {
                if byte != b'\\' {
                    return Err(Cause::Syntax);
                }
                string.phase = StringPhase::HighU;
            }
            StringPhase::HighU => {
                if byte != b'u' {
                    return Err(Cause::Syntax);
                }
                string.hex = 0;
                string.digits = 0;
                string.phase = StringPhase::LowHex;
            }
            StringPhase::Utf8 => {
                if !(0x80..=0xbf).contains(&byte) {
                    return Err(Cause::Syntax);
                }
                string.buffer[string.length as usize] = byte;
                string.length += 1;
                if string.length == string.expected {
                    // At most four bytes, independent of source string length.
                    core::str::from_utf8(&string.buffer[..string.length as usize])
                        .map_err(|_| Cause::Syntax)?;
                    string.pending = string.length;
                    string.phase = StringPhase::Normal;
                }
            }
        }
        Ok((State::String(string), true))
    }

    fn sort(
        &mut self,
        mut sort: Sort,
        budget: &mut WorkBudget,
    ) -> Result<(State<'a>, bool), Cause> {
        if sort.index == self.storage.len(Site::Edges) || self.storage.len(Site::Edges) < 2 {
            return Ok((State::Ranges(0), true));
        }
        if sort.pivot.is_none() {
            sort.pivot = Some(self.storage.edge(sort.index));
            sort.position = sort.index;
            return Ok((State::Sort(sort), true));
        }
        let pivot = sort.pivot.unwrap();
        match sort.mode {
            1 => {
                self.storage
                    .set_edge(sort.position, self.storage.edge(sort.position - 1));
                sort.position -= 1;
                sort.mode = 0;
            }
            2 => {
                self.storage.set_edge(sort.position, pivot);
                sort.index += 1;
                sort.pivot = None;
                sort.mode = 0;
            }
            _ => {
                if sort.position == 0 {
                    sort.mode = 2;
                } else {
                    let prior = self.storage.edge(sort.position - 1);
                    let order = if prior.original_index != pivot.original_index {
                        Some(prior.original_index.cmp(&pivot.original_index))
                    } else if self.storage.node(prior.original_index as usize).kind
                        != Kind::Object as u32
                    {
                        Some(Ordering::Equal)
                    } else {
                        if sort.compare.is_none() {
                            sort.compare = Some(KeyCompare {
                                left: self.key(prior),
                                right: self.key(pivot),
                                index: 0,
                                byte: None,
                            });
                        }
                        if budget.remaining(W::CodecInputBytes) == 0 {
                            return Ok((State::Sort(sort), false));
                        }
                        self.compare(sort.compare.as_mut().unwrap(), budget)?
                    };
                    if let Some(order) = order {
                        sort.compare = None;
                        sort.mode = if order == Ordering::Greater { 1 } else { 2 };
                    }
                }
            }
        }
        Ok((State::Sort(sort), true))
    }

    fn duplicates(
        &mut self,
        mut duplicates: Duplicates,
        budget: &mut WorkBudget,
    ) -> Result<(State<'a>, bool), Cause> {
        if let Some(previous) = duplicates.discard.take() {
            let mut edge = self.storage.edge(previous);
            edge.original_index = DEAD;
            self.storage.set_edge(previous, edge);
            return Ok((State::Duplicates(duplicates), true));
        }
        if duplicates.node == self.storage.len(Site::Nodes) {
            self.phase = Phase::Reachability;
            return Ok((State::Mark(self.root), true));
        }
        let node = self.storage.node(duplicates.node);
        if node.kind != Kind::Object as u32 || duplicates.position == node.edge_count as usize {
            duplicates.node += 1;
            duplicates.position = 0;
            duplicates.previous = None;
        } else {
            let current = node.first_edge as usize + duplicates.position;
            if let Some(previous) = duplicates.previous {
                if duplicates.compare.is_none() {
                    duplicates.compare = Some(KeyCompare {
                        left: self.key(self.storage.edge(previous)),
                        right: self.key(self.storage.edge(current)),
                        index: 0,
                        byte: None,
                    });
                }
                if budget.remaining(W::CodecInputBytes) == 0 {
                    return Ok((State::Duplicates(duplicates), false));
                }
                if let Some(order) = self.compare(duplicates.compare.as_mut().unwrap(), budget)? {
                    if order == Ordering::Equal {
                        duplicates.discard = Some(previous);
                    }
                    duplicates.compare = None;
                    duplicates.previous = Some(current);
                    duplicates.position += 1;
                }
            } else {
                duplicates.previous = Some(current);
                duplicates.position += 1;
            }
        }
        Ok((State::Duplicates(duplicates), true))
    }
}

fn whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
}
fn arena_cause(error: ArenaError) -> Cause {
    match error {
        ArenaError::Arithmetic => Cause::Arithmetic,
        ArenaError::Limit => Cause::Memory,
        ArenaError::Allocation => Cause::Allocation,
    }
}

/// Owns exactly the three sealed arenas, with no source or traversal lifetime.
pub struct OwnedValue {
    storage: Storage<()>,
    trace: Trace,
}
impl OwnedValue {
    pub fn view(&self) -> View<'_> {
        View {
            owner: self,
            node: 0,
        }
    }
    pub fn footprint(&self) -> Footprint {
        self.storage.footprint()
    }
    pub fn trace(&self) -> Trace {
        self.trace
    }
    pub fn allocations(&self) -> u64 {
        self.storage.allocations()
    }
    pub fn releases(&self) -> u64 {
        self.storage.releases()
    }
}

#[derive(Clone, Copy)]
pub struct View<'a> {
    owner: &'a OwnedValue,
    node: usize,
}
impl<'a> View<'a> {
    fn record(self) -> Node {
        self.owner.storage.nodes()[self.node]
    }
    pub fn kind(self) -> Kind {
        match self.record().kind {
            0 => Kind::Null,
            1 => Kind::False,
            2 => Kind::True,
            3 => Kind::String,
            4 => Kind::Number,
            5 => Kind::Array,
            6 => Kind::Object,
            7 => Kind::Entry,
            _ => unreachable!(),
        }
    }
    pub fn text(self) -> Option<&'a str> {
        if !matches!(self.kind(), Kind::String | Kind::Number) {
            return None;
        }
        let node = self.record();
        core::str::from_utf8(
            &self.owner.storage.bytes()
                [node.first_byte as usize..(node.first_byte + node.byte_count) as usize],
        )
        .ok()
    }
    pub fn len(self) -> usize {
        self.record().edge_count as usize
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
    pub fn child(self, index: usize) -> Option<View<'a>> {
        let node = self.record();
        if index >= node.edge_count as usize {
            return None;
        }
        let edge = self.owner.storage.edges()[node.first_edge as usize + index];
        Some(View {
            owner: self.owner,
            node: edge.target as usize,
        })
    }
    pub fn member(self, index: usize) -> Option<(&'a str, View<'a>)> {
        if self.kind() != Kind::Object {
            return None;
        }
        let entry = self.child(index)?;
        Some((entry.child(0)?.text()?, entry.child(1)?))
    }
    pub fn get(self, key: &str) -> Option<View<'a>> {
        (0..self.len()).find_map(|index| {
            let (name, value) = self.member(index)?;
            (name == key).then_some(value)
        })
    }
}
