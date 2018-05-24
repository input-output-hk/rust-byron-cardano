use std;
use std::fmt;
use std::string::String;

use ansi_term::Colour;

static DISPLAY_INDENT_SIZE: usize = 4; // spaces
static DISPLAY_INDENT_LEVEL: usize = 0; // beginning starts at zero
static DISPLAY_USE_COLOR: bool = false; // no color for display implementations

type Assoc = (String, Val);

// core pretty-printer type which facilitates homogenous ordered key-value pairs
pub enum Val {
    Single(Box<fmt::Display>),
    Pairs(Option<Colour>, Vec<Assoc>),
}

// XXX: this class is related to "into" somehow
pub trait Pretty {
    fn to_pretty(&self) -> Val;
}

fn longest_key_length(pairs: &Vec<Assoc>) -> usize {
    pairs
        .iter()
        .fold(0, |longest, (key, _)| std::cmp::max(longest, key.len()))
}

fn fmt_key(
    key: &String,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
    col: Option<Colour>,
    key_width: usize,
) -> fmt::Result {
    // XXX: measuring width is broken for ansi strings, so do alignment before painting
    let aligned_key = format!("{:<kw$}", key, kw = key_width,);
    let painted_key = match col {
        Some(color) => color.paint(aligned_key),
        None => aligned_key.into(),
    };
    write!(
        f,
        "{:>iw$}- {}:",
        "",
        painted_key,
        iw = indent_size * indent_level,
    )
}

fn fmt_val(
    val: &Val,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
    use_color: bool,
) -> fmt::Result {
    match val {
        // write inline
        Val::Single(_) => {
            write!(f, " ")?;
            fmt_pretty(val, f, indent_size, indent_level, use_color)?;
            write!(f, "\n")
        }
        // write on the next line
        Val::Pairs(_, _) => {
            write!(f, "\n")?;
            fmt_pretty(val, f, indent_size, indent_level, use_color)
        }
    }
    // XXX: DRY up the duplicate calls to `fmt_pretty`?
}

fn fmt_pretty(
    p: &Val,
    f: &mut fmt::Formatter,
    indent_size: usize,
    indent_level: usize,
    use_color: bool,
) -> fmt::Result {
    match p {
        // format pretty-val as a terminal
        Val::Single(display) => write!(f, "{}", display),
        // format pretty-val as a set of  key-vals
        Val::Pairs(color, pairs) => {
            let key_width = longest_key_length(pairs);
            pairs
                .iter()
                .fold(Ok(()), |prev_result, (key, val)| match prev_result {
                    // return an error
                    err @ Err(_) => err,
                    // continue writing
                    Ok(()) => {
                        let col = color.and_then(|c| if use_color { Some(c) } else { None });
                        fmt_key(key, f, indent_size, indent_level, col, key_width)?;
                        fmt_val(val, f, indent_size, indent_level + 1, use_color)
                    }
                })
        }
    }
}

// implement display for a Val without color
impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_pretty(
            self,
            f,
            DISPLAY_INDENT_SIZE,
            DISPLAY_INDENT_LEVEL,
            DISPLAY_USE_COLOR,
        )
    }
}

// implement display for anything which implements Pretty
impl fmt::Display for Pretty {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_pretty(
            &self.to_pretty(),
            f,
            DISPLAY_INDENT_SIZE,
            DISPLAY_INDENT_LEVEL,
            DISPLAY_USE_COLOR,
        )
    }
}

// format any Val to a string with color
pub fn format_val(p: &Val, indent_size: usize) -> String {
    // internal helper to get a formatter for calling `fmt_pretty`
    struct Fmt<F>(F)
    where
        F: Fn(&mut fmt::Formatter) -> fmt::Result;

    impl<F> fmt::Display for Fmt<F>
    where
        F: Fn(&mut fmt::Formatter) -> fmt::Result,
    {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.0(f)
        }
    }
    format!("{}", Fmt(|f| fmt_pretty(p, f, indent_size, 0, true)))
}

pub fn format(p: &Pretty, indent_size: usize) -> String {
    format_val(&p.to_pretty(), indent_size)
}

#[cfg(test)]
mod tests {
    use ansi_term::Colour;
    use std::vec::Vec;

    use pretty::Val::*;
    use pretty::*;

    #[test]
    fn test_display_single() {
        assert_eq!(format!("{}", Single(Box::new(123))), "123");
    }
    #[test]
    fn longest_key_length_works() {
        let mut input = Vec::new();
        input.push(("name".to_string(), Single(Box::new("zaphod"))));
        input.push(("age".to_string(), Single(Box::new(42))));
        assert_eq!(longest_key_length(&input), 4);
    }
    #[test]
    fn test_display_flat_pairs() {
        let mut input = Vec::new();
        input.push(("name".to_string(), Single(Box::new("zaphod"))));
        input.push(("age".to_string(), Single(Box::new(42))));
        assert_eq!(
            format!("{}", Pairs(Some(Colour::Red), input)),
            "\
- name: zaphod
- age : 42
"
        );
    }
    #[test]
    fn test_display_nested_pairs() {
        let mut nested = Vec::new();
        nested.push(("name".to_string(), Single(Box::new("zaphod"))));
        nested.push(("age".to_string(), Single(Box::new(42))));
        let mut input = Vec::new();
        input.push(("character".to_string(), Pairs(Some(Colour::Blue), nested)));
        input.push(("crook".to_string(), Single(Box::new("yes"))));
        assert_eq!(
            format!("{}", Pairs(Some(Colour::Red), input)),
            "\
- character:
    - name: zaphod
    - age : 42
- crook    : yes
"
        );
    }
    #[test]
    fn test_format_no_color() {
        let input = vec![("name".to_string(), Single(Box::new("zaphod")))];
        assert_eq!(format_val(&Pairs(None, input), 4), "- name: zaphod\n");
    }
    #[test]
    fn test_format_color_flat_pairs() {
        let mut input = Vec::new();
        input.push(("name".to_string(), Single(Box::new("zaphod"))));
        input.push(("age".to_string(), Single(Box::new(42))));
        assert_eq!(
            format_val(&Pairs(Some(Colour::Red), input), 4),
            "\
- \u{1b}[31mname\u{1b}[0m: zaphod
- \u{1b}[31mage \u{1b}[0m: 42
"
        );
    }
}
