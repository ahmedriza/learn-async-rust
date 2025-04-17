use actix_web::{App, HttpServer, Responder, get, rt::time::sleep, web};
use std::{
    env,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

static COUNTER: AtomicUsize = AtomicUsize::new(1);

#[get("/{delay}/{message}")]
async fn delay(path: web::Path<(u64, String)>) -> impl Responder {
    let (delay_ms, message) = path.into_inner();
    let count = COUNTER.fetch_add(1, Ordering::SeqCst);
    println!("#{count} - {delay_ms}ms: {message}");
    sleep(Duration::from_millis(delay_ms)).await;
    message
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let url = env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("localhost"));

    HttpServer::new(|| App::new().service(delay))
        .bind((url, 7070))?
        .run()
        .await
}
