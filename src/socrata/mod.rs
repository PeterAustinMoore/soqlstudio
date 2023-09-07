use regex::Regex;
use reqwest::header;
use std::error::Error;
use tokio;
use serde_derive::{Deserialize, Serialize};

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


// pub fn get_data<T>(url: String, username:String, password: String) -> Result<T, Box<dyn Error>> {
//     let result = tokio::runtime::Builder::new_multi_thread()
//         .enable_all()
//         .build()
//         .unwrap()
//         .block_on(async {
//             let mut headers = header::HeaderMap::new();
//             headers.insert(
//                 header::CONTENT_TYPE,
//                 header::HeaderValue::from_static("application/json"),
//             );
// 
//             let client = reqwest::Client::builder()
//                 .default_headers(headers)
//                 .build()
//                 // .basic_auth(username.as_str(), Some(password.as_str()))
//                 .unwrap();
//             let response = client.get(url).send().await.unwrap().json::<T>().unwrap().await.unwrap();
// 
//             Ok(response)
//         });
// 
//     result
// }
