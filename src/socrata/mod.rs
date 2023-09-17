use regex::Regex;
use serde_derive::{Deserialize, Serialize};
pub mod data;
pub mod analysis;

#[derive(Deserialize, Serialize, Debug)]
pub struct ExplainQuery {
    #[serde(alias="explainPlan")]
    pub explain_plan: String
}

fn sanitize(q: &str) -> String {
    let rm_spaces = Regex::new(r"[\s]+").unwrap();
    let a = q.replace("\n", " ");
    let b = a.as_str().replace("\t", "");
    let c = b.as_str().trim();
    let d = rm_spaces.replace_all(c, " ");
    d.to_string()
}
pub fn make_query(domain: &str, dataset: &str, query: &str) -> String {
    let query_string = format!("https://{}/resource/{}.csv?$query={}", domain, dataset, sanitize(query));
    query_string
}

pub fn make_analyze_url(domain: &str, dataset: &str, query: &str) -> String {
    let query_string = format!("https://{}/api/views/{}/query_info?analyze=true&query={}", domain, dataset, sanitize(query));
    query_string
}
