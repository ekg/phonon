// Test module to check method visibility
#[cfg(test)]
mod test {
    use crate::pattern::Pattern;
    use crate::pattern_ops::*;

    #[test]
    fn test_methods_available() {
        let p: Pattern<String> = Pattern::pure("test".to_string());

        // This should compile if degrade_by is available
        let _degraded = p.clone().degrade_by(0.5);

        // This should compile if late is available
        let _late = p.clone().late(0.1);

        assert!(true);
    }
}