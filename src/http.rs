use embassy_net::{tcp::TcpSocket, Ipv4Address, Stack};
use embassy_sync::pubsub::WaitResult;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use heapless::String;
use numtoa::NumToA;

use crate::{channel::TEMP_CHANNEL, sensors::TempMessage};

const POST_ENDPOINT_IP: &str = env!("POST_ENDPOINT_IP");
const POST_ENDPOINT_PORT: &str = env!("POST_ENDPOINT_PORT");
const TIMEOUT_SECS: u64 = 30;

#[embassy_executor::task]
pub async fn post_updates(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    let mut subscriber = TEMP_CHANNEL
        .dyn_subscriber()
        .expect("creating subscriber for temperature updates");

    let endpoint_ip = parse_ip(POST_ENDPOINT_IP);
    let endpoint_port = POST_ENDPOINT_PORT.parse::<u16>().expect("parsing port");

    log::info!(
        "Connecting to IP: {:?} Port: {}",
        endpoint_ip,
        endpoint_port
    );

    loop {
        match subscriber.next_message().await {
            WaitResult::Message(temp) => {
                log::info!("Received temperature update: {:?}", temp);
                post_data(stack, endpoint_ip, endpoint_port, &temp).await;
            }
            WaitResult::Lagged(lag_count) => {
                log::error!(
                    "We are lagging our subscription of temperature updates: {}",
                    lag_count
                );
            }
        }
    }
}

fn parse_ip(ip: &str) -> Ipv4Address {
    let mut octets = [0; 4];
    for (i, octet) in ip.split('.').enumerate() {
        octets[i] = octet.parse().expect("parsing IP address");
    }
    Ipv4Address::new(octets[0], octets[1], octets[2], octets[3])
}

async fn post_data(
    stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>,
    ip: Ipv4Address,
    port: u16,
    message: &TempMessage,
) {
    let mut rx_buf = [0; 8192];
    let mut tx_buf = [0; 8192];
    let mut socket = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(TIMEOUT_SECS)));

    let r = socket.connect((ip, port)).await;
    if let Err(e) = r {
        log::error!("Failed to connect to server: {:?}", e);
        return;
    }

    log::debug!("connected...");

    let request = create_http_post_request(message);

    if let Err(_) = request {
        log::error!("Failed to create HTTP request");
        return;
    }
    let request = request.unwrap();

    let r = write_all(&mut socket, &request.into_bytes()).await;
    if let Err(e) = r {
        log::error!("Failed to write to server: {:?}", e);
        return;
    }

    let mut buf = [0; 8192];
    let n = match socket.read(&mut buf).await {
        Ok(0) => {
            log::error!("Server closed connection");
            return;
        }
        Ok(n) => n,
        Err(e) => {
            log::error!("Failed to read from server: {:?}", e);
            return;
        }
    };

    log::info!(
        "response from server: {}",
        core::str::from_utf8(&buf[..n])
            .expect("utf8 from server")
            .lines()
            .next()
            .unwrap_or("")
    );
}

async fn write_all(socket: &mut TcpSocket<'_>, buf: &[u8]) -> Result<(), embassy_net::tcp::Error> {
    let mut buf = buf;
    while !buf.is_empty() {
        match socket.write(buf).await {
            Ok(0) => panic!("write() returned Ok(0)"),
            Ok(n) => buf = &buf[n..],
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn create_http_post_request(message: &TempMessage) -> Result<String<1024>, ()> {
    let mut request = String::new();

    let payload = create_json_payload(message)?;

    request.push_str("POST /data HTTP/1.1\r\n")?;
    request.push_str("Content-Type: application/json\r\n")?;

    request.push_str("Content-Length: ")?;
    let mut buf = [0u8; 5];
    request.push_str(payload.len().numtoa_str(10, &mut buf))?;

    request.push_str("\r\n\r\n")?;
    request.push_str(payload.as_str())?;

    Ok(request)
}

fn create_json_payload(message: &TempMessage) -> Result<String<128>, ()> {
    let mut payload = String::new();
    payload.push_str("{ \"temperature\": [")?;

    let mut buffer = ryu::Buffer::new();

    for i in 0..message.len() {
        let t = buffer.format(message[i]);
        payload.push_str(t)?;
        if i != message.len() - 1 {
            payload.push_str(",")?;
        }
    }

    payload.push_str("]}")?;

    Ok(payload)
}
