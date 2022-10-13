# Notes

## Axum

* Versions
  * 0.5
  * 0.6.0-rc.2
* Great [collection of examples](https://github.com/tokio-rs/axum/tree/main/examples)
* Makes full use of Tokio stack (e.g. tracing, tower)
* IntoResponse, Extractors
* Strange error on wrong param order

## Rocket

* Versions
  * 0.4.11
  * 0.5.0-rc.2
* Diesel
* Much older, develops slower
* Great collection of examples
* Heavily based on macros
  * Rater good errors
* Figment for config
* Poor logging story https://github.com/SergioBenitez/Rocket/issues/21 (uses log internally)
* Fairings instead of tower

## SeaORM

* https://github.com/SeaQL/sea-orm
