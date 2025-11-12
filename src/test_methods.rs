#![allow(unused_assignments, unused_mut)]
// Test module to check method visibility
#[cfg(test)]
mod test {
    use crate::pattern::Pattern;

    #[test]
    fn test_methods_available() {
        let p: Pattern<String> = Pattern::pure("test".to_string());

        // This should compile if degrade_by is available
        let _degraded = p.clone().degrade_by(Pattern::pure(0.5));

        // This should compile if late is available
        let _late = p.clone().late(Pattern::pure(0.1));

        assert!(true);
    }
}
