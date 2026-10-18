/// Монотонний час від старту плати, однаковий для всіх потоків.
pub fn now_ms() -> u64 {
    (unsafe { esp_idf_svc::sys::esp_timer_get_time() } / 1000) as u64
}
