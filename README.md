# Zerometry

It's like [`geo_types::Geometry`] except it's not.
The purpose of this crate is to provide zero-copy mapping on [`geo_types::Geometry`] that:
- Can be stored as single values on disk in a database like LMDB
- Lets us compute the relation between two shapes without deserializing anything in RAM
- Is relatively quick
- Is small

It was made for [cellulite](https://github.com/meilisearch/cellulite), a geo database
tailored specifically for [meilisearch](https://github.com/meilisearch/meilisearch).
The main operations it needs to do are:
- Store individual geometries in the values of an LMDB database
- Find shapes that intersect or are partially or strictly contained within another shape

## Usage

### How to serialize stuff

The entry point of the lib is the `write_from_geometry` method available on all types.
Let's say you want to store a point. You have to first get your [`geo_types::Point`] type,
find the equivalent in zerometry, which is called a [`Zoint`], and call the
[`Zoint::write_from_geometry`] method. This method will fill a vector of bytes.
From this buffer you can then use the [`Zoint::from_bytes`] method and get the actual
structure you can manipulate and compare to other shapes.
At this point, keep in mind that the structure only contains a reference to your buffer,
so you won't be able to modify it. 

```rust
use zerometry::Zoint;

// First get our point
let point = geo_types::Point::new(12.0, 13.0);

// Initialize the buffer that will store the point
let mut buffer = Vec::new();
// Serialize the point to the zerometry format in the buffer
Zoint::write_from_geometry(&mut buffer, &point).unwrap();
// Make a zoint out of the buffer
let zoint = unsafe { Zoint::from_bytes(&buffer) };
assert_eq!(zoint.x(), 12.0);
assert_eq!(zoint.y(), 13.0);
```

### How to query stuff

All operations between shapes are done through the [`RelationBetweenShapes`] trait.

The general idea behind the zerometry relation trait is that you **ask** for multiple things
at the same time.
And you get an answer that describes everything you asked all at once.
This is close to the [`geo::Relate`] trait.
The main difference is that you have to specify which information you're looking for before
calling it.

```rust
use zerometry::{Zoint, Zolygon, RelationBetweenShapes, InputRelation};
use geo_types::polygon;

let point = geo_types::Point::new(0.0, 0.0);
let polygon = polygon![(x: -1.0, y: -1.0), (x: 1.0, y: -1.0), (x: 1.0, y: 1.0), (x: -1.0, y: 1.0)];

let mut buffer = Vec::new();
Zoint::write_from_geometry(&mut buffer, &point).unwrap();
let zoint = unsafe { Zoint::from_bytes(&buffer) };
let mut buffer = Vec::new();
Zolygon::write_from_geometry(&mut buffer, &polygon).unwrap();
let zolygon = unsafe { Zolygon::from_bytes(&buffer) };

// Let's say we just want to know if the point is contained in the polygon,
// we could write
let relation = InputRelation { contains: true, ..InputRelation::default() };
// The we can ask the relation between our two shape with the `relation` method:
let relation = zolygon.relation(&zoint, relation);
assert_eq!(relation.contains, Some(true));
```
