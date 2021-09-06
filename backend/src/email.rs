/// Handle email

use std::time;
use std::sync::{Arc, mpsc};
use tokio::task;
use tokio::sync::Mutex;
use lettre::transport::smtp::{authentication, client};
use lettre::message::{MultiPart, Mailbox};
use lettre::address::AddressError;
use lettre::{Address, Transport};

const EMAIL_PERIOD: u64 = 10; // seconds between trying to send email

/*
#[derive(Debug, Clone)]
pub enum EmailError {
    MessageBuildFailure, MessageSendFailure, InvalidAddress
}

impl fmt::Display for EmailError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error {:?}", *self)
    }
}
*/

pub fn parse_addr(addr: &str) -> Result<Mailbox, AddressError> {
    let address: Address = addr.parse()?;
    Ok(Mailbox::new(None, address))
}

#[derive(Clone)]
pub struct Email {
    email_tx: Arc<Mutex<mpsc::Sender<Action>>>,
    //handler_thread: task::JoinHandle<()>,
}

#[derive(Clone)]
struct InThreadData {
    smtp_server: String,
    smtp_port: u16,
    smtp_username: String,
    smtp_password: String,
    base_reg_msg: lettre::message::MessageBuilder,
    frontend_loc_str: String,
}

#[derive(Clone)]
pub struct RegisterData {
    pub dest_addr: String,
    pub reg_key: String,
}

#[derive(Clone)]
pub enum Action {
    SendRegAcct(RegisterData),
}

impl Email {
    pub fn new(smtp_server: String, smtp_port: u16, smtp_username: String,
            smtp_password: String, email_from: String, frontend_loc_str: String)
        -> Self
    {
        let from_addr = match parse_addr(&email_from.clone()) {
            Ok(addr) => addr,
            Err(err) => panic!("Failed to parse email addr {} - {}",
                email_from, err),
        };

        let base_reg_msg = lettre::Message::builder()
            .from(from_addr)
            .subject("Running Stream: Verify Your Account");


        let in_thread_data = InThreadData {
            smtp_server,
            smtp_port,
            smtp_username,
            smtp_password,
            base_reg_msg,
            frontend_loc_str,
        };

        // Create channel
        let (email_tx_base, email_rx) = mpsc::channel();

        // Allow email_tx to work nicely with tokio threads and sync
        let email_tx = Arc::new(Mutex::new(email_tx_base));

        // Spawn blocking task
        let _handler_thread = task::spawn_blocking(move || {
            Self::handle_emails(in_thread_data, email_rx)
        });

        Self {
            email_tx,
            //handler_thread,
        }
    }

    pub async fn please(&self, action: Action) -> () {
        // Send the message to the email handler thread
        match self.email_tx.lock().await.send(action) {
            Ok(_) => (),
            Err(err) => {panic!("Email request failed! Dying: {}", err);},
        }
    }

    fn handle_emails(dat: InThreadData, email_rx: mpsc::Receiver<Action>) -> () {
        let sleep_time = time::Duration::from_secs(EMAIL_PERIOD);

        let creds = authentication::Credentials::
            new(dat.smtp_username.clone(), dat.smtp_password.clone());

        let tls_params = match client::TlsParameters::
            new(dat.smtp_server.clone())
        {
            Ok(params) => params,
            Err(err) => panic!("Failed building tls params: {}", err),
        };

        let smtp = match lettre::SmtpTransport::relay(&dat.smtp_server) {
            Ok(builder) => builder
                .credentials(creds)
                .tls(client::Tls::Required(tls_params))
                .port(dat.smtp_port)
                .build(),
            Err(err) => panic!("Failed building smtp: {}", err),
        };

        loop {
            std::thread::sleep(sleep_time);
            
            while let Some(msg) = match email_rx.try_recv() {
                Ok(msg) => Some(msg), 
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => { 
                    panic!("Email sender disconnected!");
                },
            } {
                match msg {
                    Action::SendRegAcct(reg_dat) => 
                        Self::send_reg_acct(smtp.clone(), dat.clone(), reg_dat),
                }
            }
        }
    }

    fn send_reg_acct(smtp: lettre::SmtpTransport, dat: InThreadData,
            reg_dat: RegisterData)
        -> ()
    {
        let text_msg = format!("Welcome to Running Stream - build your own Roku channel!  Please paste the following link into your browser to complete registration {}/?val_code={} - if you did not attempt to register at Running Stream please just delete this email.", dat.frontend_loc_str, reg_dat.reg_key);
        let html_msg = format!("<p>Welcome to Running Stream - build your own Roku channel!</p>  <p><a href=\"{}/?val_code={}\">Please click here to complete registration</a></p>  <p>If you did not attempt to register at Running Stream please just delete this email.</p>", dat.frontend_loc_str, reg_dat.reg_key);

        let dest_addr_addr = match parse_addr(&reg_dat.dest_addr) {
            Ok(addr) => addr,
            Err(err) => {
                println!("Failed to parse email addr {} - {}",
                    reg_dat.dest_addr, err);
                return;
            },
        };

        let msg = match dat.base_reg_msg.clone()
            .to(dest_addr_addr)
            .multipart(MultiPart::alternative_plain_html(
                text_msg,
                html_msg,
            ))
        {
            Ok(val) => val,
            Err(err) => {
                println!("Failed to build message: {:?}", err);
                return;
            },
        };

        match smtp.send(&msg) {
            Ok(_) => {
                println!("Registration email sent successfully");
            },
            Err(e) => {
                println!("Error sending registration email: {:?}", e);
            },
        };
    }
}
