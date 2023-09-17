#[allow(dead_code)]
pub enum AnalysisChannel {
    Data(usize)
}

pub enum AnalysisErrCause {
    Data(String)
}

pub enum AnalysisContainer {
    Data(String)
}

#[derive(Default)]
pub struct AnalysisResponseData {
    pub data: Option<String>,
    pub is_running: bool,
    pub error: Option<String>
}

impl AnalysisResponseData {
    pub fn set_data(&mut self, data: String) {
        self.error.take();
        self.data = Some(data);
    }

    pub fn set_error(&mut self, e: impl ToString) {
        self.error = Some(e.to_string());
    }
}
