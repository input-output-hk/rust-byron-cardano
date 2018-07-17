use super::ColorChoice;

/// Configuration for the output options
pub struct Config {
    /// when to display color or not
    pub color: ColorChoice,
    /// allow user to require no output
    ///
    /// Warning, this does not hide potential logging
    pub quiet: bool,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            color: ColorChoice::Auto,
            quiet: false,
        }
    }
}
