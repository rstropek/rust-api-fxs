use regex::Regex;
use spin_sdk::{
    http::Request,
};
use todo_logic::{Pagination, TodoItem, TodoStore};

// Rather naive, manual extractors. Anybody wants to write a framework for that? 😉

pub fn extract_db(req: &Request) -> TodoStore {
    if let Some(db) = req
        .headers()
        .get_all("cookie")
        .into_iter()
        .find(|c| c.to_str().unwrap().starts_with("Session="))
    {
        let db = db.to_str().unwrap();
        let re = Regex::new(r"; db=([a-zA-Z0-9]+)").unwrap();
        let mut cap = re.captures_iter(db);
        if let Some(re) = cap.next() {
            let re = &re[1];
            let db = base64::decode(re).unwrap();
            return TodoStore::from_hashmap(serde_json::from_str(std::str::from_utf8(&db).unwrap()).unwrap());
        }
    }

    TodoStore::default()
}

pub fn extract_pagination(req: &Request) -> Pagination {
    let query = req.uri().query().unwrap_or("");
    let mut pagination = Pagination::default();

    for pair in query.split('&').filter(|s| !s.is_empty()) {
        let mut parts = pair.split('=');
        let key = parts.next().unwrap();
        let value = parts.next().unwrap();

        match key {
            "offset" => pagination.offset = value.parse().map(Some).unwrap_or(None),
            "limit" => pagination.limit = value.parse().map(Some).unwrap_or(None),
            _ => {},
        }
    }

    pagination
}

pub fn extract_todo_item(req: &Request) -> TodoItem {
    let body = req.body().as_ref().unwrap();
    serde_json::from_str(std::str::from_utf8(body.as_ref()).unwrap()).unwrap()
}

pub fn extract_id(req: &Request) -> usize {
    let path = req.uri().path().to_string();
    let re = Regex::new(r"/todos/([0-9]+)").unwrap();
    let mut cap = re.captures_iter(&path);
    let re = cap.next().unwrap();
    let re = &re[1];
    re.parse().unwrap()
}
