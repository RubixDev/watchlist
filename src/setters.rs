use std::collections::{btree_map::Entry, hash_map, BTreeMap};

use actix_web::{
    delete, patch, post, put,
    web::{Data, Json, Path, Query},
    HttpResponse, Responder,
};
use chrono::NaiveDate;
use openlibrsry::OlId;

use crate::{
    schema::{Book, Movie, Rating, Reading, Tag},
    AppState,
};

#[post("/api/movie")]
async fn post_movie(data: Data<AppState>, Json(movie): Json<Movie>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.movies.entry(movie.tmdb_id) {
        Entry::Vacant(entry) => {
            entry.insert(movie);
        }
        Entry::Occupied(_) => {
            return HttpResponse::Conflict().body("a movie with that ID is already present")
        }
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[patch("/api/movie")]
async fn patch_movie(data: Data<AppState>, Json(movie): Json<Movie>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.movies.entry(movie.tmdb_id) {
        Entry::Vacant(_) => {
            return HttpResponse::NotFound()
                .body(format!("movie with ID {} does not exist", movie.tmdb_id))
        }
        Entry::Occupied(mut entry) => {
            *entry.get_mut() = movie;
        }
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[delete("/api/movie/{id}")]
async fn delete_movie(data: Data<AppState>, id: Path<u64>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    if data_lock.movies.remove(&id).is_none() {
        return HttpResponse::NotFound().body(format!("movie with ID {id} does not exist"));
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[post("/api/tag")]
async fn post_tag(data: Data<AppState>, Json(tag): Json<Tag>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    let tag_id = loop {
        let new_id = rand::random::<u32>();
        if !data_lock.tags.contains_key(&new_id) {
            break new_id;
        }
    };
    let new_tag = Tag { id: tag_id, ..tag };
    let resp = HttpResponse::Ok().json(&new_tag);
    data_lock.tags.insert(tag_id, new_tag);
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => resp,
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[patch("/api/tag")]
async fn patch_tag(data: Data<AppState>, Json(tag): Json<Tag>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.tags.entry(tag.id) {
        hash_map::Entry::Vacant(_) => {
            return HttpResponse::NotFound().body(format!("tag with ID {} does not exist", tag.id))
        }
        hash_map::Entry::Occupied(mut entry) => {
            *entry.get_mut() = tag;
        }
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[delete("/api/tag/{id}")]
async fn delete_tag(data: Data<AppState>, id: Path<u32>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    if data_lock.tags.remove(&id).is_none() {
        return HttpResponse::NotFound().body(format!("tag with ID {id} does not exist"));
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[put("/api/movie/{id}/rating")]
async fn movie_put_rating(
    data: Data<AppState>,
    id: Path<u64>,
    Json(rating): Json<Rating>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.movies.get_mut(&id) {
        Some(movie) => {
            let insert_index = match movie.ratings.binary_search_by_key(&rating.date, |w| w.date) {
                Ok(_) => {
                    return HttpResponse::Conflict().body(format!(
                        "movie with ID {id} already has a rating set for {}",
                        rating.date.format("%Y-%m-%d")
                    ))
                }
                Err(index) => index,
            };
            movie.ratings.insert(insert_index, rating);
        }
        None => return HttpResponse::NotFound().body(format!("movie with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[patch("/api/movie/{id}/rating")]
async fn movie_patch_rating(
    data: Data<AppState>,
    id: Path<u64>,
    Json(rating): Json<Rating>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.movies.get_mut(&id) {
        Some(movie) => match movie.ratings.binary_search_by_key(&rating.date, |w| w.date) {
            Ok(idx) => {
                movie.ratings[idx] = rating;
            }
            Err(_) => {
                return HttpResponse::NotFound().body(format!(
                    "movie with ID {id} has no rating set for {}",
                    rating.date.format("%Y-%m-%d")
                ))
            }
        },
        None => return HttpResponse::NotFound().body(format!("movie with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[delete("/api/movie/{id}/rating")]
async fn movie_delete_rating(
    data: Data<AppState>,
    id: Path<u64>,
    Json(date): Json<NaiveDate>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.movies.get_mut(&id) {
        Some(movie) => match movie.ratings.binary_search_by_key(&date, |w| w.date) {
            Ok(idx) => {
                movie.ratings.remove(idx);
            }
            Err(_) => {
                return HttpResponse::NotFound().body(format!(
                    "movie with ID {id} has no rating set for {}",
                    date.format("%Y-%m-%d")
                ))
            }
        },
        None => return HttpResponse::NotFound().body(format!("movie with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[post("/api/book")]
async fn post_book(data: Data<AppState>, Json(book): Json<Book>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.entry(book.olid) {
        hash_map::Entry::Vacant(entry) => {
            entry.insert(book);
        }
        hash_map::Entry::Occupied(_) => {
            return HttpResponse::Conflict().body("a book with that ID is already present")
        }
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[patch("/api/book")]
async fn patch_book(data: Data<AppState>, Json(book): Json<Book>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.entry(book.olid) {
        hash_map::Entry::Vacant(_) => {
            return HttpResponse::NotFound()
                .body(format!("book with ID {} does not exist", book.olid))
        }
        hash_map::Entry::Occupied(mut entry) => {
            *entry.get_mut() = book;
        }
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[delete("/api/book/{id}")]
async fn delete_book(data: Data<AppState>, id: Path<OlId>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    if data_lock.books.remove(&id).is_none() {
        return HttpResponse::NotFound().body(format!("book with ID {id} does not exist"));
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[put("/api/book/{id}/reading")]
async fn book_add_reading(data: Data<AppState>, id: Path<OlId>) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.get_mut(&id) {
        Some(book) => book.readings.push(Reading {
            pages_read: BTreeMap::new(),
            rating: None,
        }),
        None => return HttpResponse::NotFound().body(format!("book with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[delete("/api/book/{id}/reading/{idx}")]
async fn book_delete_reading(
    data: Data<AppState>,
    id: Path<OlId>,
    idx: Path<usize>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.get_mut(&id) {
        Some(book) => {
            if *idx >= book.readings.len() {
                return HttpResponse::NotFound().body(format!("book with ID {id} has no reading with index {idx}"));
            }
            book.readings.remove(*idx);
        }
        None => return HttpResponse::NotFound().body(format!("book with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[patch("/api/book/{id}/reading/{idx}")]
async fn book_reading_set_for_date(
    data: Data<AppState>,
    id: Path<OlId>,
    idx: Path<usize>,
    Query(date): Query<NaiveDate>,
    Query(pages): Query<u16>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.get_mut(&id) {
        Some(book) => {
            if *idx >= book.readings.len() {
                return HttpResponse::NotFound().body(format!("book with ID {id} has no reading with index {idx}"));
            }
            if pages == 0 {
                book.readings[*idx].pages_read.remove(&date);
            } else {
                book.readings[*idx].pages_read.insert(date, pages);
            }
        }
        None => return HttpResponse::NotFound().body(format!("book with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[put("/api/book/{id}/reading/{idx}/rating")]
async fn book_reading_set_rating(
    data: Data<AppState>,
    id: Path<OlId>,
    idx: Path<usize>,
    Json(rating): Json<Rating>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.get_mut(&id) {
        Some(book) => {
            if *idx >= book.readings.len() {
                return HttpResponse::NotFound().body(format!("book with ID {id} has no reading with index {idx}"));
            }
            book.readings[*idx].rating = Some(rating);
        }
        None => return HttpResponse::NotFound().body(format!("book with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}

#[delete("/api/book/{id}/reading/{idx}/rating")]
async fn book_reading_delete_rating(
    data: Data<AppState>,
    id: Path<OlId>,
    idx: Path<usize>,
) -> impl Responder {
    let mut data_lock = data.0.lock().await;
    match data_lock.books.get_mut(&id) {
        Some(book) => {
            if *idx >= book.readings.len() {
                return HttpResponse::NotFound().body(format!("book with ID {id} has no reading with index {idx}"));
            }
            book.readings[*idx].rating = None;
        }
        None => return HttpResponse::NotFound().body(format!("book with ID {id} does not exist")),
    }
    match crate::save_to_disk(&data_lock).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().body("Failed to save new data to disk"),
    }
}
