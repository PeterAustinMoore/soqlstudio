use eframe::{egui, CreationContext};
use egui::{RichText, FontId};
use flowync::{
    error::{Compact, IOError},
    CompactFlower, CompactHandle, IntoResult,
};
use egui_dock::{DockArea, NodeIndex, Style, Tree};
use csv;
use std::str;
use reqwest::{Client, Response};
use tokio::runtime;
use tokio::time::{Instant, Duration};

mod config;
use config::{get_config, set_config, Config};
mod socrata;
use socrata::{make_query, make_analyze_url};
use socrata::data::{Channel, Container, ErrCause, ResponseData};
use socrata::analysis::{AnalysisChannel, AnalysisContainer, AnalysisErrCause, AnalysisResponseData};
mod syntaxhighlight;

const PPP: f32 = 1.25;

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "SoQL Studio",
        options,
        Box::new(|ctx| Box::new(SoqlStudio::new(ctx))),
    );
}

type DataFlower = CompactFlower<Channel, Container, ErrCause>;
type DataFlowerHandle = CompactHandle<Channel, Container, ErrCause>;
type AnalysisFlower = CompactFlower<AnalysisChannel, AnalysisContainer, AnalysisErrCause>;

struct SoqlStudio {
    rt: runtime::Runtime,
    flower: DataFlower,
    analysis_flower: AnalysisFlower,
    get_data: bool,
    btn_label_next: String,
    csv_data: ResponseData,
    analysis_data: AnalysisResponseData,
    domain: String,
    username: String,
    password: String,
    current_query: String,
    dataset: String,
    url: String,
    query_duration: Duration,
}

impl SoqlStudio {
    fn new(ctx: &CreationContext) -> Self {
        ctx.egui_ctx.set_pixels_per_point(PPP);
        let c = get_config();
        Self {
            rt: runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
            flower: DataFlower::new(1),
            analysis_flower: AnalysisFlower::new(2),
            get_data: true,
            btn_label_next: "Run Query".into(),
            csv_data: Default::default(),
            analysis_data: Default::default(),
            username: c.username,
            password: c.password,
            domain: c.domain,
            dataset: c.dataset,
            current_query: c.query,
            url: "".into(),
            query_duration: Duration::new(0, 0),
        }
    }

    async fn fetch_data(url: String, username: String, password: String, handle: &DataFlowerHandle) -> Result<Container, IOError> {
        let start = Instant::now();
        // Build a client
        let client = Client::builder()
            // Needed to set UA to get image file, otherwise reqwest error 403
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
            .to_str()?
            .to_owned();
        
        
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
            let err = response.text().await;
            let t = format!("Expected  CSV; found {}: {:?}", content_type, err).into();
            Err(t)
        }
    }

    

    fn spawn_fetch_data(&mut self) {
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
        println!("Making call to: {}", self.url);
        // Set error to None
        self.csv_data.error.take();
        // Show query progress
        self.csv_data.is_running = true;
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
            match Self::fetch_data(url, username, password, &handle).await {
                Ok(container) => handle.success(container),
                Err(e) => handle.error(ErrCause::Data(format!("{:?}", e))),
            }
        });
    }

    fn reset_fetch(&mut self) {
        // Handle logical accordingly
        self.csv_data.repair();
        if self.get_data && self.flower.is_canceled() {
            if self.csv_data.seed > 1 {
                self.csv_data.seed -= 1;
            }
            self.btn_label_next = "Retry?".into();
        } else if !self.get_data && self.flower.is_canceled() {
            self.csv_data.seed += 1;
        } else {
            self.btn_label_next = "Run Query".into();
        }
    }

    fn spawn_analyze_query(&mut self) {
        let new_config = Config {
            username: self.username.to_owned(),
            password: self.password.to_owned(),
            domain: self.domain.to_owned(),
            dataset: self.dataset.to_owned(),
            query: self.current_query.to_owned()
        };
        set_config(new_config);
        self.url = make_analyze_url(self.domain.as_str(), self.dataset.as_str(), self.current_query.as_str());
        println!("{}", self.url);
        // Set error to None
        self.csv_data.error.take();
        // Show query progress
        self.csv_data.is_running = true;
        // Get flower handle
        let handle = self.analysis_flower.handle();
        let url = self.url.to_owned();
        let username = self.username.to_owned();
        let password = self.password.to_owned();
        self.rt.spawn(async move {
            // Don't forget to activate flower here
            handle.activate();
            // Start fetching
            match Self::fetch_analysis(url, username, password).await {
                Ok(container) => handle.success(container),
                Err(e) => handle.error(AnalysisErrCause::Data(format!("{:?}", e))),
            }
        });
    }

    async fn fetch_analysis(url: String, username: String, password: String) -> Result<AnalysisContainer, IOError> {
        let client = reqwest::Client::new();
        let response: Response = client
            .get(url)
            .basic_auth(username, Some(password))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(AnalysisContainer::Data("Hello".into()))
        } else {
            Err("Bad Call".into())
        }
    }
}

impl eframe::App for SoqlStudio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Labels
        let app_header = RichText::new("SoQL Studio").font(FontId::proportional(60.0)).color(egui::Color32::WHITE);
        let settings_header = RichText::new("Settings").font(FontId::proportional(40.0));
        let username_label = RichText::new("Username: ").font(FontId::proportional(25.0));
        let password_label = RichText::new("Password: ").font(FontId::proportional(25.0));
        let domain_label = RichText::new("Domain: ").font(FontId::proportional(25.0));
        let id_label = RichText::new("Dataset ID: ").font(FontId::proportional(25.0));
        
        egui::TopBottomPanel::new(egui::panel::TopBottomSide::Top, "header").show(ctx, |ui| {
            ui.heading(app_header);
        });

        egui::SidePanel::new(egui::panel::Side::Left, "id_source").show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading(settings_header);
            ui.label(username_label);
            ui.add(
                egui::TextEdit::singleline(&mut self.username)
                    .font(FontId::proportional(25.0))
                    .desired_width(375.0)
                
            );
            ui.label(password_label);
            ui.add(
                egui::TextEdit::singleline(&mut self.password)
                    .font(FontId::proportional(25.0))
                    .password(true).desired_width(375.0)
            );
            ui.label(domain_label);
            ui.add(
                egui::TextEdit::singleline(&mut self.domain)
                    .font(FontId::proportional(25.0))
                    .desired_width(375.0)
            );
            ui.label(id_label);
            ui.add(
                egui::TextEdit::singleline(&mut self.dataset)
                    .font(FontId::proportional(25.0))
                    .desired_width(375.0)
            );

        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                let mut layout_job =
                    syntaxhighlight::highlight(ui.ctx(), string, "sql");
                layout_job.wrap.max_width = wrap_width;
                ui.fonts().layout_job(layout_job)
            };
            if self.flower.is_active() {
                let mut fetch_data_finalized = false;
                self.flower
                    .extract(|message| {
                        match message {
                            Channel::Data(b) => {
                                self.csv_data.tmp_file_size += b;
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
                                self.csv_data.set_data(data);
                                fetch_data_finalized = true;
                            }
                            Err(Compact::Suppose(err)) => {
                                // Get specific error message.
                                match err {
                                    ErrCause::Elapsed(_e) => {
                                        self.query_duration = Duration::new(0, 0);
                                    }
                                    ErrCause::Data(e) => {
                                        self.csv_data.set_error(e);
                                    }
                                }
                                fetch_data_finalized = true;
                            }
                            // Handle stuff if tokio runtime panicked as well,
                            // but don't do that and stay calm is highly encouraged.
                            Err(Compact::Panicked(err)) => {
                                self.csv_data.set_error(err);
                                fetch_data_finalized = true;
                            }
                        }
                    });

                if fetch_data_finalized {
                    self.reset_fetch();
                }
            }
            if self.analysis_flower.is_active() {
                let mut fetch_analysis_finalized = false;
                self.analysis_flower
                    .extract(|message| {
                        match message {
                            // Not using this for anything yet
                            AnalysisChannel::Data(_) => {}
                        }
                    })
                    .finalize(|result| {
                        match result {
                            Ok(AnalysisContainer::Data(data)) => {
                                self.analysis_data.set_data(data);
                                fetch_analysis_finalized = true;
                            }
                            Err(Compact::Suppose(err)) => {
                                // Get specific error message.
                                match err {
                                    AnalysisErrCause::Data(e) => {
                                        self.analysis_data.set_error(e);
                                    }
                                }
                                fetch_analysis_finalized = true;
                            }
                            // Handle stuff if tokio runtime panicked as well,
                            // but don't do that and stay calm is highly encouraged.
                            Err(Compact::Panicked(err)) => {
                                self.analysis_data.set_error(err);
                                fetch_analysis_finalized = true;
                            }
                        }
                    });

                if fetch_analysis_finalized {
                    self.reset_fetch();
                }
            }

            ui.horizontal(|query_box| {
                query_box.set_height(600.0);
                egui::ScrollArea::vertical().max_height(900.0).show(query_box, |query_box| {
                    query_box.add(
                        egui::TextEdit::multiline(&mut self.current_query)
                            // .font(egui::TextStyle::Monospace) // for cursor height
                            .font(egui::TextStyle::Heading)
                            .code_editor()
                            .desired_rows(80)
                            .lock_focus(true)
                            .desired_width(f32::INFINITY)
                            .layouter(&mut layouter),
                    );
                });
            });
            // Action Buttons
            ui.horizontal(|action_buttons| {
                if action_buttons.button(egui::RichText::new(&self.btn_label_next).font(egui::FontId::proportional(30.0))).clicked() {
                    if self.flower.is_active() {
                        if !self.get_data {
                            self.btn_label_next = "Wait we are still fetching...".into();
                        } else {
                            self.flower.cancel();
                        }
                    } else {
                        // Refetch next image
                        self.csv_data.seed += 1;
                        self.spawn_fetch_data();
                        self.get_data = true;
                        self.btn_label_next = "Cancel?".into();
                    }
                }
                if action_buttons.button(egui::RichText::new("Save Query").font(egui::FontId::proportional(30.0))).clicked() {
                    //TODO: Save the query to a file
                }
                if action_buttons.button(egui::RichText::new("Run Query Analysis").font(egui::FontId::proportional(30.0))).clicked() {
                    if self.flower.is_active() {
                        if !self.get_data {
                            self.btn_label_next = "Wait we are still fetching...".into();
                        } else {
                            self.flower.cancel();
                        }
                    } else {
                        // Refetch next image
                        self.csv_data.seed += 1;
                        self.spawn_analyze_query();
                        self.get_data = true;
                        self.btn_label_next = "Cancel?".into();
                    }
                }
            });
            // The query is being executed
            if self.csv_data.is_running {
                ui.horizontal(|ui| {
                    // We don't need to call repaint since we are using spinner here.
                    ui.spinner();
                    let mut downloaded_size = self.csv_data.tmp_file_size;
                    if downloaded_size > 0 {
                        // Convert current file size in Bytes to KB.
                        downloaded_size /= 1000;
                        // Show downloaded file size.
                        ui.label(format!("Downloaded size: {} KB", downloaded_size));
                    }
                });
            }

            if let Some(err) = &self.csv_data.error {
                ui.colored_label(ui.visuals().error_fg_color, egui::RichText::new(err).font(egui::FontId::proportional(40.0)));
            }

            if let Some(csv_data) = &self.csv_data.data {
                let text_edit = egui::TextEdit::singleline(&mut self.url)
                    .desired_width(f32::INFINITY)
                    .font(FontId::proportional(20.0));
                ui.add(text_edit);
                // Query Results Table
                ui.label(egui::RichText::new("Results").font(egui::FontId::proportional(30.0)));
                egui::ScrollArea::both()
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        egui::Grid::new("my_grid").striped(true).show(ui, |grid| {
                            for (i, row) in csv_data.iter().enumerate() {
                                if i == 0 {
                                    for cell in row.iter() {
                                        grid.label(egui::RichText::new(cell).font(egui::FontId::proportional(30.0)));
                                    }
                                } else {
                                    for cell in row.iter() {
                                        grid.label(egui::RichText::new(cell).font(egui::FontId::proportional(20.0)));
                                    }
                                }
                                grid.end_row();
                            }
                        })
                    });
                // Query Stats
                ui.label(egui::RichText::new("Statistics").font(egui::FontId::proportional(30.0)));
                let file_size = self.csv_data.file_size;
                ui.label(egui::RichText::new(
                    format!("Current file size: {} KB", file_size)
                ).font(egui::FontId::proportional(20.0)));
                // Query Elapsed Text
                ui.label(egui::RichText::new(format!(
                    "Query Elapsed: {:#?}",
                    self.query_duration
                )).font(egui::FontId::proportional(20.0)));
            }
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.label("C - Peter M.");
            ui.label("")
        });
    }
}
