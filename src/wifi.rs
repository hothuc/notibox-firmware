use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::timer::EspTaskTimerService;
use std::convert::TryInto;
use esp_idf_svc::wifi::{
    AccessPointConfiguration, AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi,
};
use esp_idf_sys::EspError;
use log::{info, warn};
use std::time::Duration;
use async_std::task;
use async_std::channel::{bounded, Sender, Receiver};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WiFiStatus {
    Disconnected,
    Connecting,
    Connected { ip: [u8; 4] },
    ApMode { ip: [u8; 4] },
}

pub struct WiFiManager {
    wifi: AsyncWifi<EspWifi<'static>>,
    status_tx: Sender<WiFiStatus>,
}

impl WiFiManager {
    pub async fn new(modem: Modem) -> anyhow::Result<(Self, Receiver<WiFiStatus>)> {
        let sys_loop = EspSystemEventLoop::take()?;
        let nvs = EspDefaultNvsPartition::take()?;
        let timer_service = EspTaskTimerService::new()?;
        let wifi = AsyncWifi::wrap(
            EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
            sys_loop,
            timer_service,
        )?;
        let (status_tx, status_rx) = bounded::<WiFiStatus>(10);
        Ok((Self { wifi, status_tx }, status_rx))
    }

    pub async fn connect_or_ap_mode(&mut self, ssid: &str, password: &str) -> Result<(), EspError> {
        let station_config = Configuration::Client(ClientConfiguration {
            ssid: ssid.try_into().unwrap(),
            password: password.try_into().unwrap(),
            ..Default::default()
        });

        self.wifi.set_configuration(&station_config)?;
        
        // Tách riêng các bước await
        self.wifi.start().await?;
        match self.wifi.connect().await {
            Ok(_) => {
                info!("WiFi connected successfully");
                // Lấy IP nếu cần: self.wifi.sta_netif().get_ip_info()...
                // Ở đây minh họa giá trị giả định:
                let _ = self.status_tx.send(WiFiStatus::Connected { ip: [192, 168, 1, 100] }).await;
                Ok(())
            }
            Err(err) => {
                warn!("Station mode failed ({:?}), starting AP mode", err);
                self.start_ap_mode().await
            }
        }
    }

    pub async fn start_ap_mode(&mut self) -> Result<(), EspError> {
        let ap_config = Configuration::AccessPoint(AccessPointConfiguration {
            ssid: "Notibox-Setup".try_into().unwrap(),
            password: "notibox123".try_into().unwrap(),
            auth_method: AuthMethod::WPA2Personal,
            channel: 1,
            ..Default::default()
        });

        self.wifi.set_configuration(&ap_config)?;
        self.wifi.start().await?;
        // IP mặc định AP thường là 192.168.4.1
        let _ = self.status_tx.send(WiFiStatus::ApMode { ip: [192, 168, 4, 1] }).await;
        Ok(())
    }

    pub async fn wifi_task(mut self) -> anyhow::Result<()> {

        // Load saved credentials hoặc start AP mode
        let load_wifi_credential: Option<(String, String)> = None;
        match load_wifi_credential {
            Some((ssid, password)) => {
                self.connect_or_ap_mode(&ssid, &password).await?;
            }
            None => {
                self.start_ap_mode().await?;
            }
        }
        
        // Handle reconnection logic
        loop {
            task::sleep(Duration::from_secs(30)).await;
            // Check connection status và retry if needed
        }
    }

    // async fn load_wifi_credentials() -> Option<(String, String)> {
    //     // TODO: Implement loading from NVS or config file
    //     // For now, return None to trigger AP mode
    //     None
    // }
}