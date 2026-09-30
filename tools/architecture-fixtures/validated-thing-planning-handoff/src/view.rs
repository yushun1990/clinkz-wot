//! Borrowed contract facade; only the TD test adapter implements its backend.
use crate::Operation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedFormHref<'a> {
    Reference(&'a str),
    Template(&'a str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidatedFormHrefError {
    TemplateBase,
    Resolution,
}

/// Fixture plumbing, not an extension of the frozen public TD surface.
/// Coordinates are semantic Property ordinals / original Form indices only.
/// Every effective query is delegated to TD's existing shared test kernel.
pub trait ViewSource {
    fn id(&self) -> Option<&str>;
    fn property_count(&self) -> usize;
    fn property_name(&self, property: u32) -> &str;
    fn form_count(&self, property: u32) -> usize;
    fn href(&self, property: u32, form: u32) -> ValidatedFormHref<'_>;
    fn resolved_href(
        &self,
        property: u32,
        form: u32,
    ) -> Result<ValidatedFormHref<'_>, ValidatedFormHrefError>;
    fn content_type(&self, property: u32, form: u32) -> &str;
    fn content_coding(&self, property: u32, form: u32) -> Option<&str>;
    fn subprotocol(&self, property: u32, form: u32) -> Option<&str>;
    fn scope_count(&self, property: u32, form: u32) -> usize;
    fn scope(&self, property: u32, form: u32, index: usize) -> &str;
    fn operation_count(&self, property: u32, form: u32) -> usize;
    fn operation(&self, property: u32, form: u32, index: usize) -> Operation;
    fn security_count(&self, property: u32, form: u32) -> usize;
    fn security(&self, property: u32, form: u32, index: usize) -> &str;
    fn security_definition(&self, name: &str) -> Option<(&str, &str)>;
}

#[derive(Clone, Copy)]
pub struct ValidatedThingView<'a> {
    source: &'a dyn ViewSource,
}

#[derive(Clone, Copy)]
pub struct ValidatedPropertyView<'a> {
    source: &'a dyn ViewSource,
    ordinal: u32,
}

#[derive(Clone, Copy)]
pub struct ValidatedFormView<'a> {
    property: ValidatedPropertyView<'a>,
    original_index: u32,
}

#[derive(Clone, Copy)]
pub struct ValidatedSecuritySchemeView<'a> {
    name: &'a str,
    scheme: &'a str,
}

#[derive(Clone)]
struct Sequence<F> {
    at: F,
    next: usize,
    length: usize,
}

impl<T, F: Fn(usize) -> T> Iterator for Sequence<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.next == self.length {
            return None;
        }
        let item = (self.at)(self.next);
        self.next += 1;
        Some(item)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.length - self.next;
        (remaining, Some(remaining))
    }
}
impl<T, F: Fn(usize) -> T> ExactSizeIterator for Sequence<F> {}

fn sequence<T, F: Fn(usize) -> T + Clone>(length: usize, at: F) -> Sequence<F> {
    Sequence {
        at,
        next: 0,
        length,
    }
}

impl<'a> ValidatedThingView<'a> {
    /// Fixture adapter entry only; production construction remains private.
    pub fn from_source(source: &'a dyn ViewSource) -> Self {
        Self { source }
    }
    pub fn id(self) -> Option<&'a str> {
        self.source.id()
    }
    pub fn property(self, name: &str) -> Option<ValidatedPropertyView<'a>> {
        self.properties().find(|property| property.name() == name)
    }
    pub fn properties(
        self,
    ) -> impl ExactSizeIterator<Item = ValidatedPropertyView<'a>> + Clone + 'a {
        sequence(self.source.property_count(), move |index| {
            ValidatedPropertyView {
                source: self.source,
                ordinal: index.try_into().unwrap(),
            }
        })
    }
    pub fn security_definition(self, name: &str) -> Option<ValidatedSecuritySchemeView<'a>> {
        self.source
            .security_definition(name)
            .map(|(name, scheme)| ValidatedSecuritySchemeView { name, scheme })
    }
}

impl<'a> ValidatedPropertyView<'a> {
    pub const fn ordinal(self) -> u32 {
        self.ordinal
    }
    pub fn name(self) -> &'a str {
        self.source.property_name(self.ordinal)
    }
    pub fn form(self, original_index: u32) -> Option<ValidatedFormView<'a>> {
        (usize::try_from(original_index).ok()? < self.source.form_count(self.ordinal)).then_some(
            ValidatedFormView {
                property: self,
                original_index,
            },
        )
    }
    pub fn forms(self) -> impl ExactSizeIterator<Item = ValidatedFormView<'a>> + Clone + 'a {
        sequence(self.source.form_count(self.ordinal), move |index| {
            ValidatedFormView {
                property: self,
                original_index: index.try_into().unwrap(),
            }
        })
    }
}

impl<'a> ValidatedFormView<'a> {
    pub const fn original_index(self) -> u32 {
        self.original_index
    }
    pub fn href(self) -> ValidatedFormHref<'a> {
        self.property
            .source
            .href(self.property.ordinal, self.original_index)
    }
    pub fn resolved_href(self) -> Result<ValidatedFormHref<'a>, ValidatedFormHrefError> {
        self.property
            .source
            .resolved_href(self.property.ordinal, self.original_index)
    }
    pub fn content_type(self) -> &'a str {
        self.property
            .source
            .content_type(self.property.ordinal, self.original_index)
    }
    pub fn content_coding(self) -> Option<&'a str> {
        self.property
            .source
            .content_coding(self.property.ordinal, self.original_index)
    }
    pub fn subprotocol(self) -> Option<&'a str> {
        self.property
            .source
            .subprotocol(self.property.ordinal, self.original_index)
    }
    pub fn scopes(self) -> impl ExactSizeIterator<Item = &'a str> + Clone + 'a {
        sequence(
            self.property
                .source
                .scope_count(self.property.ordinal, self.original_index),
            move |index| {
                self.property
                    .source
                    .scope(self.property.ordinal, self.original_index, index)
            },
        )
    }
    pub fn effective_operations(self) -> impl ExactSizeIterator<Item = Operation> + Clone + 'a {
        sequence(
            self.property
                .source
                .operation_count(self.property.ordinal, self.original_index),
            move |index| {
                self.property
                    .source
                    .operation(self.property.ordinal, self.original_index, index)
            },
        )
    }
    pub fn effective_security(self) -> impl ExactSizeIterator<Item = &'a str> + Clone + 'a {
        sequence(
            self.property
                .source
                .security_count(self.property.ordinal, self.original_index),
            move |index| {
                self.property
                    .source
                    .security(self.property.ordinal, self.original_index, index)
            },
        )
    }
}

impl<'a> ValidatedSecuritySchemeView<'a> {
    pub fn name(self) -> &'a str {
        self.name
    }
    pub fn scheme(self) -> &'a str {
        self.scheme
    }
}
