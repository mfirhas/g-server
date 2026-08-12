use std::fmt;

pub trait Uri {
    fn scheme(&self) -> Option<&str>;
    fn authority(&self) -> Option<&str>;
    fn path(&self) -> &str;
    fn query(&self) -> Option<&str>;
    fn fragment(&self) -> Option<&str>;

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(scheme) = self.scheme() {
            write!(f, "{scheme}:")?;
        }

        if let Some(authority) = self.authority() {
            write!(f, "//{authority}")?;
        }

        f.write_str(self.path())?;

        if let Some(query) = self.query() {
            write!(f, "?{query}")?;
        }

        if let Some(fragment) = self.fragment() {
            write!(f, "#{fragment}")?;
        }

        Ok(())
    }
}
