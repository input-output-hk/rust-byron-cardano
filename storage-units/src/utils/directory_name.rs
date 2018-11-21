use std::ffi::OsString;
use std::{error, fmt, ops::Deref, str::FromStr};

/// Directory name with pre-validated content to avoid invalid names or names
/// we cannot use in the context of the command line interface.
///
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectoryName(String);
impl DirectoryName {
    /// validate the given [`OsString`] as a valid formatted directory name
    /// for the storage.
    ///
    /// # Errors
    ///
    /// This function can fails if the given string is of an invalid or unknown
    /// format:
    ///
    /// * It contains one of the unsupported characters: `/`, `.` or `\\`;
    /// * It contains a non unicode character.
    ///
    /// # Example
    ///
    /// ```
    /// use storage_units::utils::directory_name::DirectoryName;
    /// # use storage_units::utils::directory_name::DirectoryNameError;
    ///
    /// # fn test_function() -> Result<(), DirectoryNameError> {
    /// let path = ::std::ffi::OsString::from("This is a valid path");
    ///
    /// let _dn = DirectoryName::new(path)?; // succeed
    /// # Ok(())
    /// # }
    ///
    /// # test_function().unwrap();
    /// ```
    ///
    #[inline]
    pub fn new(oss: OsString) -> Result<Self, DirectoryNameError> {
        let s = match oss.into_string() {
            Ok(s) => s,
            Err(oss) => return Err(DirectoryNameError::UnsupportedCharacters(oss)),
        };

        Self::from_string(s)
    }

    #[inline]
    fn from_string(s: String) -> Result<Self, DirectoryNameError> {
        if let Some(index) = s.find(|c: char| (c == '/') || (c == '.') || (c == '\\')) {
            Err(DirectoryNameError::InvalidCharacterAtIndex(index))
        } else {
            Ok(DirectoryName(s.to_owned()))
        }
    }
}

impl fmt::Display for DirectoryName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl Deref for DirectoryName {
    type Target = <String as Deref>::Target;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl AsRef<str> for DirectoryName {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl FromStr for DirectoryName {
    type Err = DirectoryNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s.to_owned())
    }
}

#[derive(Debug, PartialEq)]
pub enum DirectoryNameError {
    InvalidCharacterAtIndex(usize),
    UnsupportedCharacters(OsString),
}

impl fmt::Display for DirectoryNameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DirectoryNameError::InvalidCharacterAtIndex(index) => write!(
                f,
                "Invalid directory name: contains an invalid character at index {}",
                index
            ),
            DirectoryNameError::UnsupportedCharacters(oss) => write!(
                f,
                "Invalid directory name: contains invalid characters {:?}",
                oss
            ),
        }
    }
}

impl error::Error for DirectoryNameError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            DirectoryNameError::InvalidCharacterAtIndex(_) => None,
            DirectoryNameError::UnsupportedCharacters(_) => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_directory_name() {
        assert_eq!(
            DirectoryName::new("directory1".into()),
            Ok(DirectoryName("directory1".into()))
        );
        assert_eq!(
            DirectoryName::new("directory 2".into()),
            Ok(DirectoryName("directory 2".into()))
        );
        assert_eq!(
            DirectoryName::new("directory 📦".into()),
            Ok(DirectoryName("directory 📦".into()))
        );
    }

    #[test]
    fn invalid_character_in_directory_name() {
        assert_eq!(
            DirectoryName::new("directory/1".into()),
            Err(DirectoryNameError::InvalidCharacterAtIndex(9))
        );
        assert_eq!(
            DirectoryName::new("directory.2".into()),
            Err(DirectoryNameError::InvalidCharacterAtIndex(9))
        );
        assert_eq!(
            DirectoryName::new("directory\\3".into()),
            Err(DirectoryNameError::InvalidCharacterAtIndex(9))
        );
    }

    #[test]
    fn invalid_encoding_in_directory_name() {
        #[cfg(unix)]
        use std::os::unix::ffi::OsStringExt;
        #[cfg(windows)]
        use std::os::windows::ffi::OsStringExt;

        #[cfg(windows)]
        let oss = OsString::from_wide(&[0xEEEE, 0xDDDD, 0xFFFF, 0x0888][..]);
        #[cfg(unix)]
        let oss = OsString::from_vec(vec![0xfe, 0xff, 0xfe, 0xfe, 0xff, 0xff]);

        assert_eq!(
            DirectoryName::new(oss.clone()),
            Err(DirectoryNameError::UnsupportedCharacters(oss))
        );
    }
}
