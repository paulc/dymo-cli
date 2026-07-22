pub mod protocol;

use std::time::Duration;

use btleplug::api::{
    Central, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Manager, Peripheral};
use futures::StreamExt;
use uuid::Uuid;

use crate::error::{Error, Result};

const SVC_UUID:    &str = "be3dd650-2b3d-42f1-99c1-f0f749dd0678";
const WRITE_UUID:  &str = "be3dd651-2b3d-42f1-99c1-f0f749dd0678";
const NOTIFY_UUID: &str = "be3dd652-2b3d-42f1-99c1-f0f749dd0678";

pub struct PrinterInfo {
    pub name: String,
    pub address: String,
    pub peripheral: Peripheral,
}

pub async fn scan(timeout_secs: u64) -> Result<Vec<PrinterInfo>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    let adapter = adapters.into_iter().next().ok_or(Error::NoPrinters)?;

    let svc_uuid = Uuid::parse_str(SVC_UUID).unwrap();
    adapter
        .start_scan(ScanFilter { services: vec![svc_uuid] })
        .await?;

    tokio::time::sleep(Duration::from_secs(timeout_secs)).await;
    adapter.stop_scan().await?;

    let peripherals = adapter.peripherals().await?;
    let mut printers = Vec::new();

    for p in peripherals {
        if let Ok(Some(props)) = p.properties().await {
            if props.services.contains(&svc_uuid) {
                let name = props.local_name.unwrap_or_else(|| "Dymo LT-200B".into());
                let address = props.address.to_string();
                printers.push(PrinterInfo { name, address, peripheral: p });
            }
        }
    }

    Ok(printers)
}

/// Connect to a printer. Auto-selects if only one visible; errors if multiple and no addr given.
pub async fn connect(filter_addr: Option<&str>) -> Result<Peripheral> {
    let printers = scan(4).await?;
    if printers.is_empty() {
        return Err(Error::NoPrinters);
    }

    let printer = if let Some(addr) = filter_addr {
        printers
            .into_iter()
            .find(|p| p.address.eq_ignore_ascii_case(addr))
            .ok_or_else(|| Error::PrintFailed(format!("printer {} not found", addr)))?
    } else if printers.len() == 1 {
        printers.into_iter().next().unwrap()
    } else {
        return Err(Error::PrintFailed(
            "multiple printers found - use --printer <address>".into(),
        ));
    };

    printer.peripheral.connect().await?;
    printer.peripheral.discover_services().await?;
    Ok(printer.peripheral)
}

pub async fn print_image(peripheral: &Peripheral, image: &image::GrayImage) -> Result<()> {
    let chars = peripheral.characteristics();

    let write_char = chars
        .iter()
        .find(|c| c.uuid == Uuid::parse_str(WRITE_UUID).unwrap())
        .ok_or_else(|| Error::PrintFailed("write characteristic not found".into()))?
        .clone();

    let notify_char = chars
        .iter()
        .find(|c| c.uuid == Uuid::parse_str(NOTIFY_UUID).unwrap())
        .ok_or_else(|| Error::PrintFailed("notify characteristic not found".into()))?
        .clone();

    peripheral.subscribe(&notify_char).await?;
    let mut notif_stream = peripheral.notifications().await?;

    let (header, chunks) = protocol::build_print_payload(image);

    peripheral
        .write(&write_char, &header, WriteType::WithoutResponse)
        .await?;

    for chunk in chunks {
        peripheral
            .write(&write_char, &chunk, WriteType::WithoutResponse)
            .await?;
    }

    let result = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(notif) = notif_stream.next().await {
            if notif.uuid == Uuid::parse_str(NOTIFY_UUID).unwrap()
                && notif.value.len() >= 3
                && notif.value[0] == 0x1B
                && notif.value[1] == 0x52
            {
                return notif.value[2];
            }
        }
        0xFF
    })
    .await;

    peripheral.unsubscribe(&notify_char).await.ok();

    match result {
        Ok(0) | Ok(1) | Ok(3) => Ok(()),
        Ok(4)      => Err(Error::PrintFailed("cancelled".into())),
        Ok(6)      => Err(Error::PrintFailed("low battery".into())),
        Ok(7)      => Err(Error::PrintFailed("cartridge missing".into())),
        Ok(code)   => Err(Error::PrintFailed(format!("status code {}", code))),
        Err(_)     => Err(Error::PrintFailed("no response from printer (timeout)".into())),
    }
}

pub async fn disconnect(peripheral: &Peripheral) {
    let _ = peripheral.disconnect().await;
}
