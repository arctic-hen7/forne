# California — Learn Stuff

`california` is a command-line spaced reptition system to help you learn content over time, or in cramming. It's a full system for spaced repetition, and is totally extensible by algorithms that you can write yourself in a simple scripting language, giving you complete control over your learning process.

## Installation

## Usage

You can create a new deck using one of California's custom adapters (whcih you can write yourself if you use a custom file format) with this command:

``` sh
california create --format <format> ./path/to/my/source ./path/to/my/destination
```

California works by taking your input file and converting it into its own custom format, which you can update at any time as your source file changes. The format of your file can be given as one of California's [inbuilt formats], or you can provide a custom script to parse your file.

Once you've got your California file (stored as JSON), you can use it by...

# License and Disclaimer

See [`LICENSE`](./LICENSE). This project has no affiliation whatsoever with the U.S. state of California.
