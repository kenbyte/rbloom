# rBloom

[![PyPI](https://img.shields.io/pypi/v/rbloom)](https://pypi.org/project/rbloom/)
![build](https://img.shields.io/github/actions/workflow/status/kenbyte/rbloom/CI.yml)
![license](https://img.shields.io/github/license/kenbyte/rbloom)

A fast, simple and lightweight
[Bloom filter](https://en.wikipedia.org/wiki/Bloom_filter) library for
Python, implemented in Rust. It's designed to be as pythonic as
possible, mimicking the built-in `set` type where it can. While it's a new
kid on the block (this project was started in 2023), it's also currently
the fastest kid on the block by a long shot (see the section
[Benchmarks](#benchmarks)). Releases are published on
[PyPI](https://pypi.org/project/rbloom/).

## Quickstart

This library defines only one class, which can be used as follows:

```python
>>> from rbloom import Bloom
>>> bf = Bloom(200, 0.01)  # 200 items max, false positive rate of 1%
>>> bf.add("hello")
>>> "hello" in bf
True
>>> "world" in bf
False
>>> bf.update(["hello", "world"])  # "hello" and "world" now in bf
>>> other_bf = Bloom(200, 0.01)

### add some items to other_bf

>>> third_bf = bf | other_bf  # third_bf now contains all items in
                              # bf and other_bf
>>> third_bf = bf.copy()
... third_bf.update(other_bf)  # same as above
```

Unlike BF's in real life, you can have as many of these as you want,
and they all work well together!

For the full API, see the section [Documentation](#documentation).

## Installation

On almost all platforms, simply run:

```sh
pip install rbloom
```

If you're on an uncommon platform, this may cause pip to build the library
from source, which requires the Rust
[toolchain](https://www.rust-lang.org/tools/install). You can also build
`rbloom` by cloning this repository and running
[maturin](https://github.com/PyO3/maturin):

```sh
maturin build --release
```

This will create a wheel in the `target/wheels/` directory, which can
subsequently also be passed to pip.

## Why rBloom?

Why should you use this library instead of one of the other
Bloom filter libraries on PyPI?

- **Simple:** Almost all important methods work exactly like their
  counterparts in the built-in `set`
  [type](https://docs.python.org/3/library/stdtypes.html#set-types-set-frozenset).
- **Fast:** `rbloom` is implemented in Rust, which makes it
  blazingly fast. See section [Benchmarks](#benchmarks) for more
  information.
- **Lightweight:** `rbloom` has no dependencies of its own.
- **Maintainable:** This library is very concise, and it's written
  in idiomatic Rust. Even if I were to stop maintaining `rbloom` (which I
  don't intend to), it would be trivially easy for you to fork it and keep
  it working for you.

I started `rbloom` because I was looking for a simple Bloom filter
dependency for a project, but the pure Python implementations were too
slow. The only maintained fast alternative I could find,
`pybloomfiltermmap3` (which is written in C and is a great
library), failed to work on recent versions of Python (see below),
so I felt very uncomfortable using it as a dependency. I also felt like
the many thousands of lines of code in that library were a bit hard to
handle should it stop being maintained (which is what happened to the
original `pybloomfiltermmap`). However, please note that
`pybloomfiltermmap3` implements persistent filters, while `rbloom`
currently does not, so if that's something you require, you should
definitely give that library a try.

## Benchmarks

I implemented the following simple benchmark in the respective API of
each library:

```python
bf = Bloom(10_000_000, 0.01)

for i in range(10_000_000):
    bf.add(i + 0.5)  # floats because ints are hashed as themselves

for i in range(10_000_000):
    assert i + 0.5 in bf
```

This resulted in the following runtimes:

| Library                                                            | Time     | Notes                                 |
| ------------------------------------------------------------------ | -------- | ------------------------------------- |
| [rBloom](https://pypi.org/project/rbloom/)                         | 5.956s   | works out-of-the-box                  |
| [pybloomfiltermmap3](https://pypi.org/project/pybloomfiltermmap3/) | 11.280s  | surprisingly hard to get working [1]  |
| [pybloom3](https://pypi.org/project/pybloom3/)                     | 75.871s  | works out-of-the-box                  |
| [Flor](https://pypi.org/project/Flor/)                             | 128.837s | doesn't work on arbitrary objects [2] |
| [bloom-filter2](https://pypi.org/project/bloom-filter2/)           | 325.044s | doesn't work on arbitrary objects [2] |

[1] It refused to install on Python 3.11 and kept segfaulting on 3.10, so I
installed 3.7 on my machine for this benchmark.  
[2] I tested both converting to bytes and pickling, and chose the faster time.

The benchmark was run on a 2019 Dell XPS 15 7590 with an Intel Core
i5-9300H. It was run 5 times for each library, and the average time was
used.

Also note that `rbloom` is compiled against a stable ABI for
portability, and that you can get a small but measurable speedup by
removing the `"abi3-py37"` flag from `Cargo.toml` and building
it yourself.

## Documentation

This library defines only one class, the signature of which should be
thought of as follows. Note that only the first few methods differ from
the built-in `set` type:

```python
class Bloom:

    # expected_items:  max number of items to be added to the filter
    # false_positive_rate:  max false positive rate of the filter
    # hash_func:  optional argument, see section "Cryptographic security"
    def __init__(self, expected_items: int, false_positive_rate: float,
                 hash_func=__builtins__.hash)

    @property
    def size_in_bits(self) -> int  # number of buckets in the filter

    @property
    def hash_func(self) -> Callable[[Any], int]  # retrieve the hash_func
                                                 # given to __init__

    @property
    def approx_items(self) -> float    # estimated number of items in
                                       # the filter

    #                    ALL SUBSEQUENT METHODS ARE
    #  ==========  EQUIVALENT TO THE CORRESPONDING METHODS  ===========
    #                     OF THE BUILT-IN SET TYPE

    def add(self, obj)

    def __contains__(self, obj) -> bool          # check if obj in self

    def __bool__(self) -> bool                   # False if empty

    def __repr__(self) -> str                    # basic info

    def __or__(self, other: Bloom) -> Bloom      # self | other
    def __ior__(self, other: Bloom)              # self |= other
    def __and__(self, other: Bloom) -> Bloom     # self & other
    def __iand__(self, other: Bloom)             # self &= other

    # extension of __or__
    def union(self, *others: Union[Iterable, Bloom]) -> Bloom

    # extension of __ior__
    def update(self, *others: Union[Iterable, Bloom])

    # extension of __and__
    def intersection(self, *others: Union[Iterable, Bloom]) -> Bloom

    # extension of __iand__
    def intersection_update(self, *others: Union[Iterable, Bloom])

    # these implement <, >, <=, >=, ==, !=
    def __lt__, __gt__, __le__, __ge__, __eq__, __ne__(self,
                                                       other: Bloom)->bool

    def issubset(self, other: Bloom) -> bool      # self <= other

    def issuperset(self, other: Bloom) -> bool    # self >= other

    def clear(self)                               # remove all items

    def copy(self) -> Bloom                       # duplicate self
```

To prevent death and destruction, the bitwise set operations only work on
filters where all parameters are equal (including the hash functions being
the exact same object). Because this is a Bloom filter, the `__contains__`
and `approx_items` methods are probabilistic, as are all the methods that
compare two filters (such as `__le__` and `__issubset__`).

## Cryptographic security

Python's built-in hash function is designed to be fast, not maximally
collision-resistant, so if your program depends on the false positive rate
being perfectly correct, you may want to supply your own hash function.
This is especially the case when working with very large filters (more
than a few tens of millions of items) or when false positives are very
costly and could be exploited by an adversary. Just make sure that your
hash function returns an integer between -2^127 and 2^127 - 1. Feel free
to use the following example in your own code:

```python
from rbloom import Bloom
from hashlib import sha256
from pickle import dumps

def hash_func(obj):
    h = sha256(dumps(obj)).digest()
    return int.from_bytes(h[:16], "big") - 2**127

bf = Bloom(100_000_000, 0.01, hash_func=hash_func)
```

When you throw away Python's built-in hash function and start hashing
serialized representations of objects, however, you open up a breach into
the scary realm of the unpythonic:

- Numbers like `2`, `2.0` and `2 + 0j` will suddenly no longer be equal.
- Instances of classes with custom hashing logic (e.g. to stop
  caches inside instances from affecting their hashes) will suddenly
  display undefined behavior.
- Objects that can't be serialized simply won't be hashable at all.

Making you supply your own hash function in this case is a deliberate
design decision intended to show you what you're doing and prevent
you from shooting yourself in the foot.

---

**Statement of attribution:** Bloom filters were originally proposed in
[(Bloom, 1970)](https://doi.org/10.1145/362686.362692). Furthermore, this
implementation makes use of a constant recommended by
[(L'Ecuyer, 1999)](https://doi.org/10.1090/S0025-5718-99-00996-5) for
redistributing the entropy of a single hash over multiple integers using a
linear congruential generator.
