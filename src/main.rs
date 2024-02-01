use std::{collections::HashMap, usize};
use serde::{Deserialize, Serialize};
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
    let mut start = 0; 

    if let Some(input) =  params.get("start") {
        start = input.parse::<usize>().expect("Could not parse start");
    }
    
    println!("{}", start);

    let res: Vec<Question> = store.questions.values().cloned().collect();
    
    return Ok(warp::reply::json(&res));
}

async fn return_error(r: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(error) = r.find::<CorsForbidden>() {
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
    questions: HashMap<QuestionId, Question>,
}
impl Store {
    fn new() -> Self {
        Store {
            questions: Self::init(),
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

    let get_items = warp::get()
        .and(warp::path("questions"))
        .and(warp::path::end())
        .and(warp::query())
        .and(store_filter)
        .and_then(get_questions)
        .recover(return_error);
    
    let routes = get_items.with(cors);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;
}
