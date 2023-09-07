use eframe::{egui, CreationContext};
use flowync::{
    error::{Compact, IOError},
    CompactFlower, CompactHandle, IntoResult,
};
use csv;
use std::str;
use reqwest::Client;
use tokio::runtime;
use tokio::time::{Instant, Duration};
mod utils;
use utils::{Channel, Container, ErrCause, NetworkImage};
mod config;
use config::{get_config, set_config, Config};
mod socrata;
use socrata::make_query;
mod syntaxhighlight;

const PPP: f32 = 1.25;

fn main() {
    let mut options = eframe::NativeOptions::default();
    options.always_on_top = true;
    eframe::run_native(
        "SoQL Studio",
        options,
        Box::new(|ctx| Box::new(EframeTokioApp::new(ctx))),
    );
}

type TypedFlower = CompactFlower<Channel, Container, ErrCause>;
type TypedFlowerHandle = CompactHandle<Channel, Container, ErrCause>;

struct EframeTokioApp {
    rt: runtime::Runtime,
    flower: TypedFlower,
    get_data: bool,
    btn_label_next: String,
    net_image: NetworkImage,
    domain: String,
    username: String,
    password: String,
    current_query: String,
    dataset: String,
    url: String,
    query_duration: Duration,
}

impl EframeTokioApp {
    fn new(ctx: &CreationContext) -> Self {
        ctx.egui_ctx.set_pixels_per_point(PPP);
        let c = get_config();
        Self {
            rt: runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            flower: TypedFlower::new(1),
            get_data: true,
            btn_label_next: "Run".into(),
            net_image: Default::default(),
            username: c.username,
            password: c.password,
            domain: c.domain,
            dataset: c.dataset,
            current_query: c.query,
            url: "".into(),
            query_duration: Duration::new(0, 0),
        }
    }

    async fn fetch_image(url: String, username: String, password: String, handle: &TypedFlowerHandle) -> Result<Container, IOError> {
        let start = Instant::now();
        // Build a client
        let client = Client::builder()
            // Needed to set UA to get image file, otherwise reqwest error 403
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:105.0) Gecko/20100101")
            .build()?;
        let mut response = client
            .get(url)
            .basic_auth(username, Some(password))
            .send()
            .await?;

        // Get Content-Type
        let content_type = response
            .headers()
            .get("Content-Type")
            .catch("unable to get content type")?
            .to_str()?;

        if content_type.contains("text/csv") {
            let cancelation_msg = "Fetching image canceled.";
            let mut image_bytes = Vec::new();
            {
                while let Some(a_chunk) = response.chunk().await? {
                    // Handle cancelation here
                    if handle.should_cancel() {
                        return Err(cancelation_msg.into());
                    }

                    // Send chunk size as download progress
                    let progress = Channel::Data(a_chunk.len());
                    handle.send_async(progress).await;
                    a_chunk.into_iter().for_each(|x| {
                        image_bytes.push(x);
                    });
                }
            }

            // And also handle cancelation here
            if handle.should_cancel() {
                return Err(cancelation_msg.into());
            }
            let elapsed = Channel::Elapsed(start.elapsed());
            handle.send_async(elapsed).await;
            let s = match str::from_utf8(&image_bytes) {
                Ok(v) => v,
                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
            };
            let mut first_ten_records = vec![];
            let mut headers = vec![];
            let mut reader = csv::Reader::from_reader(s.as_bytes());
            match reader.headers() {
                Err(_) => headers.push(String::from("Error")),
                Ok(records) => {
                    for header in records {
                        headers.push(header.to_string())
                    }
                }
            }
            first_ten_records.push(headers);
            for (i, row) in reader.records().enumerate() {
                if i < 10 {
                    let mut row_data = vec![];
                    for cell in row.unwrap().iter() {
                        row_data.push(cell.to_string())
                    }
                    first_ten_records.push(row_data)
                } else {
                    break;
                }
            }
            let t = Container::Data(first_ten_records);
            Ok(t)
        } else {
            Err(format!("Expected  CSV; found {}", content_type).into())
        }
    }

    fn spawn_fetch_image(&mut self) {
        // Save the new config
        let new_config = Config {
            username: self.username.to_owned(),
            password: self.password.to_owned(),
            domain: self.domain.to_owned(),
            dataset: self.dataset.to_owned(),
            query: self.current_query.to_owned()
        };
        set_config(new_config);

        self.url = make_query(self.domain.as_str(), self.dataset.as_str(), self.current_query.as_str());
        println!("{}", self.url);
        // Set error to None
        self.net_image.error.take();
        // Show download image progress
        self.net_image.show_image_progress = true;
        // Get flower handle
        let handle = self.flower.handle();
        let url = self.url.to_owned();
        let username = self.username.to_owned();
        let password = self.password.to_owned();
        // Spawn tokio runtime.
        self.rt.spawn(async move {
            // Don't forget to activate flower here
            handle.activate();
            // Start fetching
            match Self::fetch_image(url, username, password, &handle).await {
                Ok(container) => handle.success(container),
                Err(e) => handle.error(ErrCause::Data(format!("{:?}", e))),
            }
        });
    }

    fn reset_fetch_image(&mut self) {
        // Handle logical accordingly
        self.net_image.repair();
        if self.get_data && self.flower.is_canceled() {
            if self.net_image.seed > 1 {
                self.net_image.seed -= 1;
            }
            self.btn_label_next = "Retry?".into();
        } else if !self.get_data && self.flower.is_canceled() {
            self.net_image.seed += 1;
        } else {
            self.btn_label_next = "Run".into();
        }
    }
}

impl eframe::App for EframeTokioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("SoQL Studio");
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job =
                    syntaxhighlight::highlight(ui.ctx(), string, "sql");
                layout_job.wrap.max_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };
            if self.flower.is_active() {
                let mut fetch_image_finalized = false;
                self.flower
                    .extract(|message| {
                        match message {
                            Channel::Data(b) => {
                                self.net_image.tmp_file_size += b;
                            }
                            Channel::Elapsed(e) => {
                                self.query_duration = e
                            }
                        }
                    })
                    .finalize(|result| {
                        match result {
                            Ok(Container::Elapsed(_)) => {}
                            Ok(Container::Data(data)) => {
                                self.net_image.set_image(data);
                                fetch_image_finalized = true;
                            }
                            Err(Compact::Suppose(err)) => {
                                // Get specific error message.
                                match err {
                                    ErrCause::Elapsed(e) => {
                                        println!("{}", e)
                                    }
                                    ErrCause::Data(e) => {
                                        // Handle if DataErr is any.
                                        println!("{}", e)
                                    }
                                }
                            }
                            // Handle stuff if tokio runtime panicked as well,
                            // but don't do that and stay calm is highly encouraged.
                            Err(Compact::Panicked(err)) => {
                                self.net_image.set_error(err);
                                fetch_image_finalized = true;
                            }
                        }
                    });

                if fetch_image_finalized {
                    self.reset_fetch_image();
                }
            }


            ui.horizontal(|app| {
                // Settings and Query Section
                app.horizontal(|settings_and_query| {
                    //Settings
                    settings_and_query.set_height(200.0);
                    settings_and_query.vertical(|settings| {
                        settings.set_width(300.0);
                        // Credentials
                        settings.horizontal(|ui| {
                            ui.label("Username: ");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |uu| {
                                uu.text_edit_singleline(&mut self.username);
                            });
                        });
                        settings.horizontal(|ui| {
                            ui.label("Password: ");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |uu| {
                                uu.add(egui::TextEdit::singleline(&mut self.password).password(true));
                            });
                        });
                        // Domain and Dataset configs
                        settings.horizontal(|ui| {
                            ui.label("Domain: ");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |uu| {
                                uu.text_edit_singleline(&mut self.domain);
                            });
                        });
                        settings.horizontal(|ui| {
                            ui.label("Dataset: ");
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |uu| {
                                uu.text_edit_singleline(&mut self.dataset);
                            });
                        });
                    });
                    settings_and_query.vertical(|query| {
                        egui::ScrollArea::vertical().max_height(200.0).show(query, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.current_query)
                                    .font(egui::TextStyle::Monospace) // for cursor height
                                    .code_editor()
                                    .desired_rows(10)
                                    .lock_focus(true)
                                    .desired_width(f32::INFINITY)
                                    .layouter(&mut layouter),
                            );
                        });
                    });
                });
            });
            ui.horizontal(|ui| {
                if ui.button(&self.btn_label_next).clicked() {
                    if self.flower.is_active() {
                        if !self.get_data {
                            self.btn_label_next = "Wait we are still fetching...".into();
                        } else {
                            self.flower.cancel();
                        }
                    } else {
                        // Refetch next image
                        self.net_image.seed += 1;
                        self.spawn_fetch_image();
                        self.get_data = true;
                        self.btn_label_next = "Cancel?".into();
                    }
                }
            });

            if self.net_image.show_image_progress {
                ui.horizontal(|ui| {
                    // We don't need to call repaint since we are using spinner here.
                    ui.spinner();
                    let mut downloaded_size = self.net_image.tmp_file_size;
                    if downloaded_size > 0 {
                        // Convert current file size in Bytes to KB.
                        downloaded_size /= 1000;
                        // Show downloaded file size.
                        ui.label(format!("Downloaded size: {} KB", downloaded_size));
                    }
                });
            }

            if let Some(err) = &self.net_image.error {
                ui.colored_label(ui.visuals().error_fg_color, err);
            }

            if let Some(csv_data) = &self.net_image.image {
                let file_size = self.net_image.file_size;
                ui.label(format!("Current file size: {} KB", file_size));
                ui.label(format!(
                    "Query Elapsed: {:#?}",
                    self.query_duration
                ));
                ui.label("URL:");
                let text_edit = egui::TextEdit::singleline(&mut self.url).desired_width(1000.0);
                ui.add(text_edit);

                egui::ScrollArea::both()
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        egui::Grid::new("my_grid").show(ui, |grid| {
                            for row in csv_data.iter() {
                                for cell in row.iter() {
                                    grid.label(cell);
                                }
                                grid.end_row();
                            }
                        })
                    });
            }
        });
    }
}
