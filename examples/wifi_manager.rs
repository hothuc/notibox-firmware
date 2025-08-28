use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use notibox_firmware::wifi::WiFiManager; // crate của bạn
use async_std::task;
use std::sync::{Mutex, Arc};

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    // Bắt buộc: khởi tạo ESP-IDF (logger, panic handler…)
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    // Lấy modem từ HAL
    let peripherals = esp_idf_hal::peripherals::Peripherals::take().unwrap();
    let modem: Modem = peripherals.modem;

    let (wifi_manager, status_rx) = WiFiManager::new(modem).await?;
    let wifi_manager = Mutex::new(wifi_manager);
    // Event loop của hệ thống
    // let sys_loop = EspSystemEventLoop::take()?;

    {
        let mut guard = wifi_manager.lock().expect("Mutex poisoned");
        guard.connect_or_ap_mode("al", "password").await?;
    }

     // xử lý status_rx
    loop {
        if let Ok(status) = status_rx.recv().await {
            println!("Status: {:?}", status);
        }
        task::yield_now().await; // nhường CPU
    }
    Ok(())
}