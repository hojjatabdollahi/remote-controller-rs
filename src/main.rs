use aws_iot_device_sdk_rust::client;
use eframe::{
    egui::{self, ScrollArea},
    epi,
};
use lazy_static::{__Deref, lazy_static};
use rumqttc::QoS;
use std::{
    collections::VecDeque,
    sync::{mpsc::Receiver, Arc, Mutex},
    thread, time,
};

use serde_json;
use serde_json::Value as JsonValue;
use serde_derive::Deserialize;

struct RCApp {
    label: String,
    iot_client: Arc<Mutex<client::AWSIoTClient>>,
    sendbtn_enabled: bool,
    in_progress: Option<Receiver<Result<String, String>>>,
    result: Option<Result<String, String>>,
    log_output: Arc<Mutex<String>>,
}

impl RCApp {
    const CLIENT_ID: &'static str = "remote-controller-dashboard";
    const CA_CERT: &'static str = "/opt/greengrass/rootCA.pem";
    const CLIENT_CERT: &'static str = "/opt/greengrass/thingCert.crt";
    const PRIVATE_KEY: &'static str = "/opt/greengrass/privKey.key";
    const IOT_ENDPOINT: &'static str = "ar0y46zddi7n5-ats.iot.us-east-1.amazonaws.com";
    const TOPIC: &'static str = "remote/homepc";
}

impl Default for RCApp {
    fn default() -> Self {
        let result = Self {
            label: "this is the default".to_owned(),
            iot_client: Arc::new(Mutex::new(
                client::AWSIoTClient::new(
                    Self::CLIENT_ID,
                    Self::CA_CERT,
                    Self::CLIENT_CERT,
                    Self::PRIVATE_KEY,
                    Self::IOT_ENDPOINT,
                )
                .unwrap(),
            )),
            sendbtn_enabled: true,
            in_progress: Default::default(),
            result: Default::default(),
            log_output: Arc::new(Mutex::new(Default::default())),
        };
        result.iot_client.lock().unwrap().subscribe(
            Self::TOPIC.to_string(),
            QoS::AtMostOnce,
            callback,
        );
        thread::sleep(time::Duration::from_secs(1));
        result
    }
}

impl epi::App for RCApp {
    fn name(&self) -> &str {
        "Ryan Remote Controller"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        if let Some(receiver) = &mut self.in_progress {
            // are we done yet?
            if let Ok(result) = receiver.try_recv() {
                self.in_progress = None;
                self.result = Some(result);
                self.sendbtn_enabled = true;
            } else {
                self.sendbtn_enabled = false;
            }
        } else {
            self.sendbtn_enabled = true;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new("Publishing to remote/homepc"));
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.set_enabled(self.sendbtn_enabled);
                    if ui.small_button("Publish").clicked() {
                        // self.sendbtn_enabled = false;
                        let repaint_singal = frame.repaint_signal();
                        let (sender, receiver) = std::sync::mpsc::channel();
                        self.in_progress = Some(receiver);
                        self.label = "Published".to_owned();
                        let iotclient = Arc::clone(&self.iot_client);
                        thread::spawn(move || {
                            for i in 0..15 {
                                let payload = format!("{{\"test\": \"Hello world {}.\"}}", i);
                                println!("Publish: {}", payload);
                                iotclient.lock().unwrap().publish(
                                    Self::TOPIC.to_string(),
                                    QoS::AtMostOnce,
                                    &payload,
                                );
                                thread::sleep(time::Duration::from_secs(1));
                            }
                            iotclient
                                .lock()
                                .unwrap()
                                .unsubscribe(Self::TOPIC.to_string());
                            sender.send(Ok("done".to_owned())).unwrap();
                            repaint_singal.request_repaint();
                        });
                        // self.sendbtn_enabled = true;
                    }
                });

                ui.label(&self.label);
                ui.separator();

                ScrollArea::auto_sized().show(ui, |ui| {
                    if let Ok(a) = self.log_output.lock() {
                        ui.code(a.deref());
                    }
                });
            });
        });
    }
}

fn callback(input: String) {
    println!("{}", input);
    ARRAY.lock().unwrap().push_back(format!("{}\n",input));
}

lazy_static! {
    static ref ARRAY: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());
}

#[derive(Debug, Deserialize)]
struct Command {
    command: String,
    data: String,
}

fn main() {

    let json_str = r#"
        {
            "command":"os",
            "data":"ls"
        }
    "#;
    let res1: JsonValue = serde_json::from_str(json_str).unwrap();
    println!("{}", res1["command"]);
    let res: Command = serde_json::from_str(json_str).unwrap();
    println!("{}", res.command);
    let app = RCApp::default();
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
