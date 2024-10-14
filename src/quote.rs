use itertools::Itertools;

fn quote_argument(arg: &str) -> String {
    let has_single = arg.contains('\'');
    let has_double = arg.contains('"');
    let has_space = arg.contains(' ');

    match (has_space, has_single, has_double) {
        (false, false, false) => arg.to_string(),
        (_, true, false) => format!("\"{}\"", arg),
        (_, false, _) => format!("'{}'", arg),
        _ => format!("'{}'", arg.replace('\'', "\\'")),
    }
}

pub fn quote_cmdline<T: AsRef<str>>(cmdline: &[T]) -> String {
    cmdline.iter().map(|s| quote_argument(s.as_ref())).join(" ")
}