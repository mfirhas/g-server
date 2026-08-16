pub trait Uri {
    fn scheme(&self) -> Option<&str>;
    fn authority(&self) -> Option<&str>;
    fn path(&self) -> &str;
    fn query(&self) -> Option<&str>;
    fn fragment(&self) -> Option<&str>;

    fn display(&self) -> String {
        let mut uri = String::new();

        if let Some(scheme) = self.scheme() {
            uri.push_str(scheme);
            uri.push(':');
        }

        if let Some(authority) = self.authority() {
            uri.push_str("//");
            uri.push_str(authority);
        }

        uri.push_str(self.path());

        if let Some(query) = self.query() {
            uri.push('?');
            uri.push_str(query);
        }

        uri
    }
}
