pub const PRODUCT_NAME: &str = "Cookbench";

pub mod domain;
pub mod state_machine;

#[cfg(test)]
mod tests {
    #[test]
    fn workspace_smoke_test() {
        assert_eq!(crate::PRODUCT_NAME, "Cookbench");
    }
}
