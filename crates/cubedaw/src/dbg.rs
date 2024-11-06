use std::{sync::LazyLock, time::Instant};

pub fn now() -> String {
    static START: LazyLock<Instant> = LazyLock::new(Instant::now);
    let elapsed = START.elapsed();
    format!(
        "{:03}.{:03}",
        elapsed.subsec_millis(),
        elapsed.subsec_nanos()
    )
}
