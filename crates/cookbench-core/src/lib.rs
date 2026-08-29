pub const PRODUCT_NAME: &str = "Cookbench";

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke_test() {
        assert_eq!(crate::PRODUCT_NAME, "Cookbench");
    }
}
