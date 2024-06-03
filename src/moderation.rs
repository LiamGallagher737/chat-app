use serde::{Deserialize, Serialize};

pub async fn is_kind_message(input: &str) -> Result<bool, reqwest::Error> {
    let client = reqwest::Client::new();
    let report: ModerationReport = client
        .post("https://despam.io/api/v1/moderate")
        .json(&ModerationRequest { input })
        .header(
            "x-api-key",
            dotenvy::var("DESPAM_API_KEY").expect("Despam api key is required"),
        )
        .send()
        .await?
        .json()
        .await?;

    Ok(report.toxic < 0.6
        && report.indecent < 0.6
        && report.threat < 0.6
        && report.offensive < 0.8
        && report.erotic < 0.6
        && report.spam < 0.8)
}

#[derive(Serialize)]
struct ModerationRequest<'a> {
    input: &'a str,
}

#[derive(Deserialize, Debug)]
struct ModerationReport {
    pub toxic: f64,
    pub indecent: f64,
    pub threat: f64,
    pub offensive: f64,
    pub erotic: f64,
    pub spam: f64,
}
