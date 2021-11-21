use std::{
    borrow::Cow,
    cell::{Cell, RefCell},
    ops::Range,
};

use druid::{
    piet::{self, PietTextLayoutBuilder, TextAttribute, TextLayoutBuilder},
    text::{self, EditableText, StringCursor},
    widget::{Scope, ScopeTransfer},
    Color, Data, Env, Widget,
};

#[derive(Clone)]
pub struct ValidatedString {
    value: String,
    valid: Cell<bool>,
}

impl Data for ValidatedString {
    fn same(&self, other: &Self) -> bool {
        self.value == other.value && self.valid.get() == other.valid.get()
    }
}

impl text::TextStorage for ValidatedString {
    fn add_attributes(&self, builder: PietTextLayoutBuilder, _: &Env) -> PietTextLayoutBuilder {
        if self.valid.get() {
            builder
        } else {
            builder.default_attribute(TextAttribute::TextColor(Color::RED))
        }
    }
}

impl piet::TextStorage for ValidatedString {
    fn as_str(&self) -> &str {
        &self.value
    }
}

impl EditableText for ValidatedString {
    fn cursor(&self, position: usize) -> Option<StringCursor> {
        self.value.cursor(position)
    }

    fn edit(&mut self, range: Range<usize>, new: impl Into<String>) {
        self.value.edit(range, new);
    }

    fn slice(&self, range: Range<usize>) -> Option<Cow<str>> {
        self.value.slice(range)
    }

    fn len(&self) -> usize {
        self.value.len()
    }

    fn prev_word_offset(&self, offset: usize) -> Option<usize> {
        self.value.prev_word_offset(offset)
    }

    fn next_word_offset(&self, offset: usize) -> Option<usize> {
        self.value.next_word_offset(offset)
    }

    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize> {
        self.value.prev_grapheme_offset(offset)
    }

    fn next_grapheme_offset(&self, offset: usize) -> Option<usize> {
        self.value.next_grapheme_offset(offset)
    }

    fn prev_codepoint_offset(&self, offset: usize) -> Option<usize> {
        self.value.prev_codepoint_offset(offset)
    }

    fn next_codepoint_offset(&self, offset: usize) -> Option<usize> {
        self.value.next_codepoint_offset(offset)
    }

    fn preceding_line_break(&self, offset: usize) -> usize {
        self.value.preceding_line_break(offset)
    }

    fn next_line_break(&self, offset: usize) -> usize {
        self.value.next_line_break(offset)
    }

    fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    fn from_str(s: &str) -> Self {
        ValidatedString {
            value: s.to_string(),
            valid: Cell::new(true),
        }
    }
}

struct MyTransfer<T, F, P> {
    format: F,
    parse: P,
    cache: RefCell<Option<(T, String)>>,
}

impl<T, F, P> ScopeTransfer for MyTransfer<T, F, P>
where
    T: Data,
    F: Fn(&mut String, &T),
    P: Fn(&str) -> Option<T>,
{
    type In = T;
    type State = ValidatedString;

    fn read_input(&self, state: &mut ValidatedString, inner: &T) {
        let mut cached = self.cache.borrow_mut();
        if cached
            .as_mut()
            .map_or(true, |(cached, _)| !cached.same(inner))
        {
            state.value.clear();
            (self.format)(&mut state.value, inner);
            state.valid.set(true);
            *cached = Some((inner.clone(), state.value.clone()));
        }
    }

    fn write_back_input(&self, state: &ValidatedString, inner: &mut T) {
        let mut cached = self.cache.borrow_mut();
        if cached
            .as_mut()
            .map_or(true, |(_, cached)| cached != &state.value)
        {
            let valid = if let Some(parsed) = (self.parse)(&state.value) {
                *inner = parsed;
                true
            } else {
                false
            };
            state.valid.set(valid);
            *cached = Some((inner.clone(), state.value.clone()));
        }
    }
}

pub fn formatted<T: Data>(
    inner: impl Widget<ValidatedString>,
    format: impl Fn(&mut String, &T) + Clone,
    parse: impl Fn(&str) -> Option<T>,
) -> impl Widget<T> {
    Scope::from_function(
        {
            let format = format.clone();
            move |val| {
                let mut buf = String::new();
                format(&mut buf, &val);
                ValidatedString {
                    value: buf,
                    valid: Cell::new(true),
                }
            }
        },
        MyTransfer {
            format,
            parse,
            cache: RefCell::new(None),
        },
        inner,
    )
}

pub fn percentage(inner: impl Widget<ValidatedString>) -> impl Widget<f64> {
    formatted(
        inner,
        |buf: &mut String, &val: &f64| {
            use std::fmt::Write;
            let _ = write!(buf, "{:.0}%", 100.0 * val);
        },
        |input: &str| {
            let parsed = input.strip_suffix('%')?.parse::<f64>().ok()?;
            (0.0..=100.0).contains(&parsed).then(|| 0.01 * parsed)
        },
    )
}
