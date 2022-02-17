use std::{
    fs::{self},
    sync::Arc,
};

use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::get,
    AddExtensionLayer, Router,
};
use serde::{Deserialize, Serialize};
use tera::{Context, Tera};
use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Template(#[from] tera::Error),
    #[error("directory not found")]
    MissDesktopDir,
    #[error("target {0} directory not found")]
    TargeDirNotFound(String),
    #[error("toml parse error on {0}")]
    TomlParse(String),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Template(err) => Html(format!("生成が失敗しました。error:{err}")).into_response(),
            Self::MissDesktopDir => Html("デスクトップが見つかりませんでした。").into_response(),
            Self::TargeDirNotFound(dir) => Html(format!(
                "デスクトップに{dir}フォルダが見つかりませんでした。"
            ))
            .into_response(),
            Self::TomlParse(path) => {
                Html(format!("{path}のフォーマットが正確ではありません。")).into_response()
            }
        }
    }
}

type Result<T> = std::result::Result<T, Error>;
#[tokio::main]
async fn main() {
    // build our application with a single route
    let raw_template = include_str!("../index.html");
    let mut index_template = Tera::default();
    index_template
        .add_raw_template("index", raw_template)
        .unwrap();
    let shared = Arc::new(index_template);
    let app = Router::new()
        .route("/", get(server_index))
        .layer(AddExtensionLayer::new(shared));
    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    println!("Hello, world!");
}

#[derive(Serialize, Deserialize)]
struct Gantt {
    title: String,
    sections: Vec<Section>,
}
#[derive(Serialize, Deserialize)]
struct Section {
    name: String,
    content: String,
}

#[derive(Serialize)]
struct Index {
    gantts: Vec<Gantt>,
}

async fn server_index(Extension(tera): Extension<Arc<Tera>>) -> Result<impl IntoResponse> {
    let desktop = dirs_next::desktop_dir().ok_or(Error::MissDesktopDir)?;
    let read_dir = desktop
        .join("gantt")
        .read_dir()
        .map_err(|_| Error::TargeDirNotFound(String::from("gantt")))?;
    let mut gantts = Vec::new();
    for entry_res in read_dir {
        let file_path = entry_res.unwrap().path();
        let content = fs::read_to_string(&file_path).unwrap();
        let gantt: Gantt = toml::from_str(&content)
            .map_err(|_| Error::TomlParse(String::from(file_path.to_str().unwrap())))?;
        gantts.push((
            String::from(file_path.file_name().unwrap().to_str().unwrap()),
            gantt,
        ));
    }
    gantts.sort_by(|(a, _), (b, _)| a.cmp(b));
    let gantts = gantts
        .into_iter()
        .map(|(_, gantt)| gantt)
        .collect::<Vec<_>>();
    let context = Index { gantts };
    let rendered = tera
        .render("index", &Context::from_serialize(&context).unwrap())
        .unwrap();
    Ok(Html(rendered))
}
