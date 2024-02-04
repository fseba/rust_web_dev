use std::{collections::HashMap, sync::Arc, usize};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use warp::{filters::{body::json, cors::CorsForbidden}, http::Method, reject::Reject, Filter, Rejection, Reply, http::StatusCode};

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Question {
    id: QuestionId,
    title: String,
    content: String,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Clone, Eq, Hash, PartialEq, Deserialize)]
struct QuestionId(String);

impl std::fmt::Display for QuestionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "id: {}", self.0)
    }
}

async fn get_questions(
    params: HashMap<String, String>,
    store: Store
) -> Result<impl Reply, Rejection> {
    if !params.is_empty() {
        let pagination = extract_pagination(params)?;

        let res: Vec<Question> = store
            .questions
            .read()
            .await
            .values()
            .cloned()
            .collect();
        
        let end_index = if pagination.end > res.len() {
            res.len()
        } else {
            pagination.end
        };

        let res = &res[pagination.start..end_index];
        return Ok(warp::reply::json(&res));
    } else {
        let res: Vec<Question> = store.questions.read().await.values().cloned().collect();
        return Ok(warp::reply::json(&res));
    }
}

async fn add_question(
    store: Store,
    question: Question
) -> Result<impl warp::Reply, warp::Rejection> {
    store.questions.write().await.insert(question.id.clone(), question);
    
    return Ok(warp::reply::with_status("Question added", StatusCode::OK));
}

async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(error) = r.find::<Error>() {
        Ok(warp::reply::with_status(
            error.to_string(), 
            StatusCode::RANGE_NOT_SATISFIABLE,
        ))
    } else if let Some(error) = r.find::<CorsForbidden>() {
        Ok(warp::reply::with_status(
            error.to_string(), 
            StatusCode::FORBIDDEN
        ))
    } else {
        Ok(warp::reply::with_status(
            "Route not found".to_string(), 
            StatusCode::NOT_FOUND))
    }
}


#[derive(Clone)]
struct Store {
    questions: Arc<RwLock<HashMap<QuestionId, Question>>>,
}
impl Store {
    fn new() -> Self {
        Store {
            questions: Arc::new(RwLock::new(Self::init())),
        }
    }

    fn add_question(mut self, question: Question) -> Self {
        self.questions.insert(question.id.clone(), question);
        return self;
    }
    
    fn init() -> HashMap<QuestionId, Question> {
        let file = include_str!("../questions.json");
        return serde_json::from_str(file).expect("can't read questions.json");
    }
}

#[derive(Debug)]
enum Error {
    ParseError(std::num::ParseIntError),
    MissingParameter,
    InvalidArgumentsOrder,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::ParseError(ref err) => {
                write!(f, "Cannot parse parameter: {}", err)
            },
            Error::MissingParameter => write!(f, "Missing parameter"),
            Error::InvalidArgumentsOrder => write!(f, "Order of arguments is invalid. 'Start' cannot be greater than 'end'"),
        }
    }
}

impl Reject for Error {}

#[derive(Debug)]
struct Pagination {
    start: usize,
    end: usize,
}

fn extract_pagination(
    params: HashMap<String, String>
) -> Result<Pagination, Error> {
    if let (Some(start), Some(end)) = (params.get("start"), params.get("end")) {

        let start = start.parse::<usize>().map_err(Error::ParseError)?;
        let end = end.parse::<usize>().map_err(Error::ParseError)?;

        if end >= start {
            Ok(Pagination { start, end })
        } else {
            Err(Error::InvalidArgumentsOrder)
        }
    } else {
        Err(Error::MissingParameter)
    }
}

#[tokio::main]
async fn main() {
    let store = Store::new();
    let store_filter = warp::any().map(move || store.clone());
    
    let cors = warp::cors()
        .allow_any_origin()
        .allow_header("content-type")
        .allow_methods(
            &[Method::PUT, Method::DELETE, Method::GET, Method::POST ]
        );

    let get_questions = warp::get()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(warp::query())
        .and(store_filter)
        .and_then(get_questions);
    
    let add_question = warp::post()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(store_filter.clone())
        .and(warp::body::json())
        .and_then(add_question);
    
    let routes = get_questions
        .or(add_question)
        .with(cors)
        .recover(return_error);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}
