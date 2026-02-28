use summer_job::job::Job;

#[test]
fn test_cron_job_builder() {
    // Test that job builder API works
    let _builder = Job::cron("0 0 * * * *");
    // If we get here without panic, the test passes
}

#[test]
fn test_fix_delay_job_builder() {
    let _builder = Job::fix_delay(60);
    // If we get here without panic, the test passes
}

#[test]
fn test_fix_rate_job_builder() {
    let _builder = Job::fix_rate(30);
    // If we get here without panic, the test passes
}

#[test]
fn test_one_shot_job_builder() {
    let _builder = Job::one_shot(10);
    // If we get here without panic, the test passes
}

#[test]
fn test_cron_job_with_data() {
    #[derive(serde::Serialize)]
    struct TestData {
        value: i32,
    }
    
    let _builder = Job::cron_with_data("0 0 * * * *", TestData { value: 42 });
    // If we get here without panic, the test passes
}

#[test]
fn test_fix_delay_job_with_data() {
    #[derive(serde::Serialize)]
    struct TestData {
        name: String,
    }
    
    let _builder = Job::fix_delay_with_data(60, TestData {
        name: "test".to_string(),
    });
    // If we get here without panic, the test passes
}

#[test]
fn test_fix_rate_job_with_data() {
    #[derive(serde::Serialize)]
    struct TestData {
        count: u32,
    }
    
    let _builder = Job::fix_rate_with_data(30, TestData { count: 100 });
    // If we get here without panic, the test passes
}

#[test]
fn test_one_shot_job_with_data() {
    #[derive(serde::Serialize)]
    struct TestData {
        id: u64,
    }
    
    let _builder = Job::one_shot_with_data(10, TestData { id: 12345 });
    // If we get here without panic, the test passes
}

#[test]
fn test_cron_expression_patterns() {
    // Valid cron expressions
    let valid_expressions = vec![
        "0 0 * * * *",           // Every hour
        "*/5 * * * * *",         // Every 5 seconds
        "0 */15 * * * *",        // Every 15 minutes
        "0 0 0 * * *",           // Daily at midnight
        "0 0 12 * * MON-FRI",    // Weekdays at noon
    ];
    
    for expr in valid_expressions {
        let _builder = Job::cron(expr);
        // If we get here without panic, the expression is accepted
    }
}

#[test]
fn test_different_delay_durations() {
    // Test various delay durations
    let delays = vec![1, 5, 60, 300, 3600];
    
    for delay in delays {
        let _builder = Job::fix_delay(delay);
        // If we get here without panic, the test passes
    }
}

#[test]
fn test_different_rate_durations() {
    // Test various rate durations
    let rates = vec![1, 5, 30, 60, 300];
    
    for rate in rates {
        let _builder = Job::fix_rate(rate);
        // If we get here without panic, the test passes
    }
}

#[test]
fn test_job_builder_type_safety() {
    // Test that the builder API maintains type safety
    let _cron_builder = Job::cron("* * * * * *");
    let _delay_builder = Job::fix_delay(10);
    let _rate_builder = Job::fix_rate(20);
    let _oneshot_builder = Job::one_shot(5);
    
    // All builders should be created without issues
}

#[test]
fn test_serializable_data() {
    #[derive(serde::Serialize)]
    struct ComplexData {
        id: u64,
        name: String,
        values: Vec<i32>,
        active: bool,
    }
    
    let data = ComplexData {
        id: 1,
        name: "test".to_string(),
        values: vec![1, 2, 3],
        active: true,
    };
    
    let _builder = Job::cron_with_data("0 0 * * * *", data);
    // If we get here, serialization works correctly
}

