use std::error::Error;
use std::sync::Arc;
use std::convert::Infallible;
use std::collections::HashMap;
use axum::{Router,
            body::Body,
            extract::{Path, Json},
            response::Response,
            routing::{get, post}
        };
use std::sync::Mutex;
use std::net::SocketAddr;
use tokio;
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Movie {
    id: String,
    name: String,
    year: u16,
    was_good: bool
}

pub struct AxumMovieManager {
    app: Option<Router>,
    known_movies: Arc<Mutex<HashMap<u64, Movie>>>
} 

impl AxumMovieManager {
    pub fn new() -> AxumMovieManager {
        AxumMovieManager {
            app: Some(Router::new()),
            known_movies: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub fn create_routes(&mut self) -> Result<(), Box<dyn Error>> {
        
        if let Some(app) = self.app.take() {
            // need to clone arcs because they are consumed by closures
            let movie_map = Arc::clone(&self.known_movies);
            let movie_map2 = Arc::clone(&self.known_movies);

            let a = app.route("/movie/:id",
                get(|Path(id): Path<u64>| async move {
                        let movies = movie_map.lock().unwrap();
                        if movies.contains_key(&id) {
                            let res = Response::builder()
                                .status(200)
                                .body(Body::from(serde_json::to_string(&movies[&id]).unwrap()))
                                .unwrap();
                            Ok::<_, Infallible>(res)
                        } else {
                            let res = Response::builder()
                                .status(421)
                                .body(Body::from("Could not find movie!"))
                                .unwrap();
                            Ok::<_, Infallible>(res)
                        }
                    }
                )
            ).route("/movie/", 
                post(|Json(movie): Json<Movie>| async move {

                        let id_u64 = movie.id.parse::<u64>().unwrap();
                        let mut movies = movie_map2.lock().unwrap();
                        if movies.contains_key(&id_u64) {
                            // TODO: check that this movie is the same as the one in the 'DB'
                            let res = Response::builder()
                                .status(420)
                                .body(Body::from("Already have this movie stored"))
                                .unwrap();

                            Ok::<_, Infallible>(res)
                        } else {
                            movies.insert(id_u64, movie);
                            let res = Response::builder()
                                .status(200)
                                .body(Body::from("Added movie to db!"))
                                .unwrap();
                            Ok::<_, Infallible>(res)
                        }
                    }
                )
            );
            self.app = Some(a);
        }

        // TODO
            // [ ] actually provide more error handling during app generation
            // [ ] cache recently seen movies - likely using another hashmap. 
            // [ ] offload db to actual db resource (S3, postgres, etc)

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Create Axum server with the following endpoints:
    // 1. GET /movie/{id} - This should return back a movie given the id
    // 2. POST /movie - this should save movie in a DB (HashMap<String, Movie>). This movie will be sent
    // via a JSON payload. 
    
    // As a bonus: implement a caching layer so we don't need to make expensive "DB" lookups, etc.
    
    let mut app = AxumMovieManager::new();
    app.create_routes().expect("Could not create routes!!");

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    if let Some(app) = app.app {
        axum_server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}