# USet and UMap
An implementation of a set and a map with unsigned integers as keys.

### Premise

I believe in using the right tools for a job. There are cases where generic data structures together with traits implementations designed to work for any kind of element held inside, and any kind of usage, result in sub-optimal performance, and unclear code.

This project is an attempt to provide more suited alternatives for `HashSet` and `HashMap` when used in situations like video games or real scene simulations. I imagined a situation when the user has to handle a few hundreds or thousands (but not billions) of data structures describing objects in the scene. They can be created, modified, and deleted fairly quickly. Instead of references, they hold identifiers to data structures in other maps. The identifiers can be simple integers, so the data structure is of known size at any point in time. This allows the user to store all of them in one or many maps where unsigned integers work as keys, retrieve those keys as sets of unsigned integers, operate on them, and finally use them to access the data structures. A set of identifiers, `USet`, is pretty lightweight and can be cloned without much effort if moving the ownership is not possible. A map, `UMap`, may be hold in one place, accessible from many others, as a form of a primitive (ECS)[https://en.wikipedia.org/wiki/Entity_component_system].

### Implementation

Implementation is very simple: 
`USet` is built around a vector of booleans. It also has an offset, which helps with memory conservation, and redundant minimum and maximum values, which greatly speed up operations on the set. If the n-th element in the vector is `true`, from the API point of view it means that the set contains the value `n - offset`.  

Accordingly, `UMap<T>` is built around a vector of elements of the type `Option<T>`. If the n-th element in the vector is `Some(t)`, from the API point of view it means that the map contains the value `t` under the identifier `n - offset`.

All this means that `USet` and `UMap` trade memory for CPU. They are not very well suited for storing huge number of entries in the memory, but for smaller amounts the benchmarks show a 3.5-4.0x performance boost over a similar functionality. 

### Usage

Contrary to `HashSet` and `HashMap`, `USet` is not a subclass of `UMap`. The two closely cooperate, but their roles are a bit different. The idiomatic use of `UMap` is to populate it with elements and then use the `query` or `keys` methods to construct `USet`s of identifiers fulfilling certain conditions. Then the `USet`s can be passed around and operated on (the implemented methods are: put, remove, union, common set, difference, and xor), and then the `UMap` can be accessed again with the resulting `USet`s, or individual identifiers, in order to read, modify, or delete data from it.

### Planned improvements

This is an initial 0.1 version of the project, so of course there's still a lot to be done! :)
1. Minimize usage of cloning within the code. This mainly applies to `UMap`. For the 0.1 version I decided to appease the borrow checker with cloning the data structures where other solutions seemed not possible, but I'm learning all the time. I hope to optimize the code a bit.
2. Consider `bitvec` for the `USet` implementation insted of `Vec<boolean>`. If it affects the performance too much, I might decide to create another implementation of `USet` (`UBitSet`?) which trades a bit of CPU power for better memory usage.
3. Work on turning `UMap` into a full-fledged ECS. Again, it might mean that I will leave the actual `UMap` as it is, with only small changes, and instead I'll create another entity with more robust functionality which will closely interact with `USet` and `UMap`.
