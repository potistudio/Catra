use crate::activity_log::emit_activity_log;
use crate::download::download_and_import;
use serde::Deserialize;
use std::io::Read;
use tiny_http::{Header, Method, Response, Server, StatusCode};
use tauri::AppHandle;

pub const PORT: u16 = 17340;

#[derive(Debug, Deserialize)]
struct DownloadRequest {
    url: String,
}

fn cors_headers() -> Vec<Header> {
    vec![
        Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap(),
        Header::from_bytes("Access-Control-Allow-Methods", "GET, POST, OPTIONS").unwrap(),
        Header::from_bytes("Access-Control-Allow-Headers", "Content-Type").unwrap(),
        Header::from_bytes("Content-Type", "application/json").unwrap(),
    ]
}

fn json_response(status: StatusCode, body: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_string(body).with_status_code(status);
    for header in cors_headers() {
        response = response.with_header(header);
    }
    response
}

fn handle_request(app: &AppHandle, mut request: tiny_http::Request) {
    let method = request.method().clone();
    let url = request.url().to_string();

    if method == Method::Options {
        let response = Response::empty(StatusCode(204));
        let mut response = response;
        for header in cors_headers() {
            response = response.with_header(header);
        }
        let _ = request.respond(response);
        return;
    }

    if method == Method::Get && url == "/health" {
        let _ = request.respond(json_response(
            StatusCode(200),
            r#"{"success":true,"service":"catra"}"#,
        ));
        return;
    }

    if method == Method::Post && url == "/download" {
        let mut body = String::new();
        if request.as_reader().read_to_string(&mut body).is_err() {
            let _ = request.respond(json_response(
                StatusCode(400),
                r#"{"success":false,"error":"Invalid request body"}"#,
            ));
            return;
        }

        let payload = match serde_json::from_str::<DownloadRequest>(&body) {
            Ok(payload) => payload,
            Err(_) => {
                let _ = request.respond(json_response(
                    StatusCode(400),
                    r#"{"success":false,"error":"Invalid JSON"}"#,
                ));
                return;
            }
        };

        if payload.url.trim().is_empty() {
            let _ = request.respond(json_response(
                StatusCode(400),
                r#"{"success":false,"error":"URL is required"}"#,
            ));
            return;
        }

        emit_activity_log(
            app,
            "info",
            "Chrome 拡張からダウンロード要求を受信",
            Some(payload.url.clone()),
        );
        download_and_import(app.clone(), payload.url);
        let _ = request.respond(json_response(
            StatusCode(202),
            r#"{"success":true,"status":"started"}"#,
        ));
        return;
    }

    let _ = request.respond(json_response(
        StatusCode(404),
        r#"{"success":false,"error":"Not found"}"#,
    ));
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        let address = format!("127.0.0.1:{PORT}");
        let server = match Server::http(&address) {
            Ok(server) => server,
            Err(error) => {
                emit_activity_log(
                    &app,
                    "error",
                    format!("拡張機能サーバーの起動に失敗: {address}"),
                    Some(error.to_string()),
                );
                return;
            }
        };

        emit_activity_log(
            &app,
            "success",
            format!("拡張機能サーバーを起動: http://{address}"),
            None,
        );

        for request in server.incoming_requests() {
            handle_request(&app, request);
        }
    });
}
