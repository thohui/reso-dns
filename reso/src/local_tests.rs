#[cfg(test)]
mod tests {
    use super::super::local::Local;
    use std::time::Duration;

    #[test]
    fn test_local_default() {
        let local = Local::default();

        assert!(!local.cache_hit);
        assert!(!local.metrics_recorded);
        assert!(!local.blocked);
    }

    #[test]
    fn test_local_time_elapsed() {
        let local = Local::default();

        std::thread::sleep(Duration::from_millis(10));

        let elapsed = local.time_elapsed();
        assert!(elapsed >= Duration::from_millis(10));
        assert!(elapsed < Duration::from_millis(100));
    }

    #[test]
    fn test_local_clone() {
        let local1 = Local {
            cache_hit: true,
            metrics_recorded: true,
            blocked: true,
            time_started: std::time::Instant::now(),
        };

        let local2 = local1.clone();

        assert_eq!(local1.cache_hit, local2.cache_hit);
        assert_eq!(local1.metrics_recorded, local2.metrics_recorded);
        assert_eq!(local1.blocked, local2.blocked);
    }

    #[test]
    fn test_local_equality() {
        let now = std::time::Instant::now();
        let local1 = Local {
            cache_hit: true,
            metrics_recorded: false,
            blocked: true,
            time_started: now,
        };

        let local2 = Local {
            cache_hit: true,
            metrics_recorded: false,
            blocked: true,
            time_started: now,
        };

        assert_eq!(local1, local2);
    }

    #[test]
    fn test_local_inequality() {
        let now = std::time::Instant::now();
        let local1 = Local {
            cache_hit: true,
            metrics_recorded: false,
            blocked: false,
            time_started: now,
        };

        let local2 = Local {
            cache_hit: true,
            metrics_recorded: false,
            blocked: true,
            time_started: now,
        };

        assert_ne!(local1, local2);
    }

    #[test]
    fn test_local_time_elapsed_immediate() {
        let local = Local::default();
        let elapsed = local.time_elapsed();

        assert!(elapsed < Duration::from_millis(10));
    }

    #[test]
    fn test_local_state_transitions() {
        let mut local = Local::default();
        assert!(!local.cache_hit);

        local.cache_hit = true;
        assert!(local.cache_hit);

        local.blocked = true;
        assert!(local.blocked);

        local.metrics_recorded = true;
        assert!(local.metrics_recorded);
    }
}