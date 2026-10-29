# Generics, Dynamic Dispatch and Static Dispatch

> Static where you can, dynamic where you must

Rust allows you to handle polymorphic code in two ways:
* **Generics / Static Dispatch**: compile-time, monomorphized per use.
* **Trait Objects / Dynamic Dispatch**: runtime vtable, single implementation.

Understanding the trade-offs lets you write faster, smaller and more flexible code.

## 6.1 [Generics](https://doc.rust-lang.org/book/ch10-00-generics.html)

Every programming language has tools for effectively handling the duplication of concepts. In Rust, one such tool is generics: abstract stand-ins for concrete types or other properties. We can express the behavior of generics or how they relate to other generics without knowing what will be in their place when compiling and running the code. 

We use generics to create definitions for items like function signatures or structs, which we can then use with many different concrete data types. Let's first look at how to define functions, structs, enums, and methods using generics. Generics can also be used to implement Type State Pattern and constrain a struct functionality to certain expected types, more on type state on [Type State](./type-state.md).

[Generics by Examples](https://doc.rust-lang.org/rust-by-example/generics.html).

### Generics Performance

You might be wondering whether there is a runtime cost when using generic type parameters. The good news is that using generic types won't make your program run any slower than it would with concrete types. Rust accomplishes this by performing monomorphization of the code using generics at compile time. Monomorphization is the process of turning generic code into specific code by filling in the concrete types that are used when compiled. The compiler checks for all occurrences of the generic parameter and generates code for the concrete types the generic code is called with.

## 6.2 Static Dispatch: `impl Trait` or `<T: Trait>`

A static dispatch is basically a constrained version of a generics, a trait bounded generic, at compile-time it is able to check if your generic satisfies the declared traits.

### ✅ Best when:
* You want **zero runtime cost**, by paying the compile time cost.
* You need **tight loops or performance**.
* Your types are **known at compile time**.
* Your are working with **single-use implementations** (monomorphized).

### 🏎️ Example: High-performance function with generic
```rust
fn specialized_sum<T: MyTrait, U: Iterator<Item = T>>(iter: U) -> T {
    iter.map(|x| x.random_mapping()).sum()
}

// or, equivalent, more modern
fn specialized_sum<T: MyTrait>(iter: impl Iterator<Item = T>) -> T {
    iter.map(|x| x.random_mapping()).sum()
}
```

This is compiled into **specialized machine code** for each usage, fast and inlined.

## 6.3 Dynamic Dispatch: `dyn Trait`

Usually dynamic dispatch is used with some kind of pointer or a reference, like `Box<dyn Trait>`, `Arc<dyn Trait>` or `&dyn trait`.

### ✅ Best when:
* You need **heterogeneous types in one collection**.
* You want runtime **plugins or hot-swappable components**.
* You want to **abstract internals behind a stable interface** (library design).

### ❌ Avoid when:
* You control the concrete types.
* You are on a **performance critical path**.
* Generics express the same logic without extra complexity.

> ❗ Closer to what you would get in an object oriented language and can have some heavy costs associated to it. Can avoid generic entirely and let you mix types that implement the same traits.

### 🚚 Example: Heterogeneous collection

```rust
trait Animal {
    fn greet(&self) -> String;
}

struct Dog;
impl Animal for Dog {
    fn greet(&self) -> String {
        "woof".to_string()
    }
}

struct Cat;
impl Animal for Cat {
    fn greet(&self) -> String {
        "meow".to_string()
    }
}

fn all_animals_greeting(animals: Vec<Box<dyn Animal>>) {
    for animal in animals {
        println!("{}", animal.greet())
    }
}
```

## 6.4 Trade-off summary

| | Static Dispatch (impl Trait) | Dynamic Dispatch (dyn Trait) |
|------------------- |------------------------------ |---------------------------------- |
| Performance | ✅ Faster, inlined | ❌ Slower: vtable indirection |
| Compile time | ❌ Slower: monomorphization | ✅ Faster: shared code |
| Binary size | ❌ Larger: per-type codegen | ✅ Smaller |
| Flexibility | ❌ Rigid, one type at a time | ✅ Can mix types in collections |
| Trait requirements | ✅ Any trait | ❌ Trait must be dyn compatible |
| Errors | ✅ Clearer | ❌ Erased types can confuse errors |

* Prefer generics/static dispatch when you control the call site and want performance.
* Use dynamic dispatch when you need abstraction, plugins or mixed types. 🚨 Runtime cost.
* If you are not sure, start with generics, trait bound them - then use `Box<dyn Trait>` when flexibility outweighs speed.

> Favor static dispatch until your trait needs to live behind a pointer.

## 6.5 🚨 Trait Objects Ergonomics

* Borrow `&dyn Trait` without allocating when ownership is unnecessary; use
  `Box<dyn Trait>` at a boundary that needs ownership.
* Use `Arc<dyn Trait>` for shared access across threads.
* **Avoid boxing too early**. Don't box inside structs unless you are sure it'll be beneficial or is required (recursive).
```rust
// ✅ Use generics when possible
struct Renderer<B: Backend> {
    backend: B
}

// ❌ Premature Boxing
struct Renderer {
    backend: Box<dyn Backend> // Boxing too early
}
```
* **Dyn compatibility** (formerly object safety) applies to the whole trait:
    * Its supertraits are dyn compatible, it does not require `Self: Sized`, and it has
      no associated constants or generic associated types.
    * Dispatchable methods have no type parameters, use `Self` only in their receiver,
      and have neither `async fn` nor return-position `impl Trait` return types.
      Supported receivers include references, `Box`, `Rc`, `Arc` and their allowed
      `Pin` forms; plain by-value `self` is not dispatchable.
    * Methods outside that contract can require `where Self: Sized`, excluding them
      from object dispatch while allowing the remaining trait to be dyn compatible.
      Test the intended `dyn Trait` usage, auto traits and lifetimes.
    ```rust
    // ✅ Dyn compatible
    trait Runnable {
        fn run(&self);
    }

    // ❌ Not dyn compatible
    trait Factory {
        fn create<T>() -> T; // no Self: Sized exemption for this generic function
    }
    ```
    > See the reference on [dyn compatibility](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility) for the full rules.

## 6.6 Trait and closure contracts

- Define traits for polymorphism or useful extension methods, not merely to organize
  functions. Extension traits conventionally use an `Ext` suffix. Keep parameter and
  lifetime names descriptive when their role is otherwise unclear.
- Require the calling contract you need: `FnOnce` accepts the widest set of closures
  when called once; use `FnMut` for repeated mutable calls and `Fn` for shared calls.
- Keep required trait methods minimal and provide defaults for derivable behavior.
  Even adding a default method can create downstream method ambiguity; review compatibility.
- Avoid reflection-shaped designs when an enum or trait expresses the boundary.
  `type_name` is diagnostic text, not a stable identifier; `Any`/downcasting is an
  intentional type-erasure boundary with a `'static` requirement.
