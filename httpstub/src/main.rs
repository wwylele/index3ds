use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer};

fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let url = std::env::var("HTTPS_URL").expect("HTTPS_URL");

    let redirect = move |req: HttpRequest| -> HttpResponse {
        HttpResponse::MovedPermanently()
            .header(
                actix_web::http::header::LOCATION,
                format!(
                    "{}{}{}{}",
                    url,
                    req.path(),
                    if req.query_string().is_empty() {
                        ""
                    } else {
                        "?"
                    },
                    req.query_string()
                ),
            )
            .finish()
    };
    HttpServer::new(move || {
        let redirect = redirect.clone();
        App::new().default_service(web::route().to(redirect))
    })
    .bind("0.0.0.0:80")?
    .run()
}
