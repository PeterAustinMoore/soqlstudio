use std::time::Duration;


#[allow(dead_code)]
pub enum Channel {
    Data(usize),
    Elapsed(Duration),
}

#[allow(dead_code)]
pub enum ErrCause {
    Data(String),
    Elapsed(String),
}

#[allow(dead_code)]
pub enum Container {
    Data(Vec<Vec<String>>),
    Elapsed(Duration),
}

#[derive(Default)]
pub struct NetworkImage {
    pub image: Option<Vec<Vec<String>>>,
    pub file_size: usize,
    pub tmp_file_size: usize,
    pub show_image_progress: bool,
    pub error: Option<String>,
    pub seed: usize,
}

impl NetworkImage {
    pub fn set_image(&mut self, image: Vec<Vec<String>>) {
        self.error.take();
        self.image = Some(image);
    }

    pub fn set_error(&mut self, e: impl ToString) {
        self.error = Some(e.to_string());
    }

    pub fn repair(&mut self) {
        // Convert final file size in Bytes to KB.
        if self.tmp_file_size >= 1000 {
            self.tmp_file_size /= 1000;
            self.file_size = self.tmp_file_size;
        }
        self.show_image_progress = false;
        self.tmp_file_size = 0;
    }
}
