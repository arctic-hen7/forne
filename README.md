# Forne — Learn Stuff

Forne is a *Turing-complete spaced repetition engine* to help you learn stuff your way. What does that mean? Well, there are a few parts:

- **Turing-complete**: Forne is fully programmable using [Rhai](https://rhai.rs), a modern scripting language
- **Spaced repetition** is the process of learning content over a period of time by periodically reviewing it
- **Engine**: Forne is a library that can be used by anyone to add spaced repetition into their own apps

But, Forne is also a *command-line interface* (CLI), meaning you can get started with it straight away even if you've never coded in your life!

## Why does this exist?

There are plenty of spaced repetition apps out there: Anki, Quizlet, SuperMemo, and a million others. But all of them do a few things: they lock you into one algorithm, they make importing from your own notes hard, and they don't let you cram.

Let's be realistic: if we're learning stuff purely for our own interests, then spaced repetition might work superbly for us, but, if there's a test coming up at the end of term, we probably won't get everything done just right, because life happens. If you're using Anki and you have a test coming up, best of luck to you, because cramming the last few terms and reviewing everything rapidly is a pain. Using an app like Quizlet is the opposite: unless you're prepared to cough up for the paid version, you'll be stuck with an algorithm that qualifies as spaced repetition only in fairyland.

The bottom line here is that we need a system that can do both: that can let you learn in the long-term and also help you cram terms for tests, while letting you test yourself on all the terms in a set, keeping track of the ones you find difficult. And, it should be able to be synced between your devices however you like to sync things, without requiring you to have an account with some third-party, and without locking you in to some service that will probably eventually go bust anyway.

Forne is designed to solve these problems by being fully scriptable in two ways:

1. You can write custom programs in a simple scripting language to import your notes into Forne for review, and
2. You can write and tweak custom learning algorithms.

By default, Forne comes with a small (but growing) library of spaced repetition and cramming algorithms, which can be used for any set imported into the program (although once you create a set, it will be locked to the chosen method, and you'll have to create another version of it if you want to use a different method). These learning methods can store their own arbitrary data about every single term in your set, and they can execute arbitrary (but securely sandboxed) code, meaning you can implement everything from a simple "show it twice and she'll be right" algorithm to a scientifically-backed artificially intelligent algorithm.

## Installation

*Note: Forne is both a library that developers can use to add spaced repetition to their apps, and a CLI that users can use to learn things. This section is about the CLI, and the library is documented [here](https://docs.rs/forne).*

You can install Forne from [the releases page](https://github.com/arctic-hen7/forne/releases), or with `cargo install forne`, easy as that!

## Usage

### Creating a new set

``` sh
forne new <source-file> <output-file>.json -a <path-to-my-adapter> -m <method>
```

Creating a new Forne set is fairly simple, but it involves understanding two key concepts: *methods* and *adapters*. The former refer to the learning algorithms you use to study a set, which are fully customisable and tweakable. Forne comes with a few that are inbuilt (see [this directory](https://github.com/arctic-hen7/forne/tree/main/src/methods) for a list), any of which you can specify after `-m`, or you can provide a path to a custom Rhai script, which will be used instead. More on creating custom methods later.

You'll also need to specify the path to a custom adapter script after `-a`, which is the Rhai script that will create a set out of your source file. Forne doesn't provide any of these by default, because everyone's file formats are so diverse, but you can take a look [here](https://github.com/arctic-hen7/forne/tree/main/common_adapters) to see some common ones, or to gain inspiration. More on creating custom adapters later.

### Listing the cards in a set

``` sh
forne list <set-file>.json
```

You can easily list all the *cards* in a Forne set with the above command, providing it the JSON file produced by `forne new` (as above). However, Forne has two special properties that can be listed on cards: they can be marked *difficult* or *starred*, which have different meanings. Difficult cards are automatically marked by the learning method you choose, while cards are starred if you get them wrong in a test. To list only difficult cards, add `-t difficult` to the end of the above command, or `-t starred` if you only want to see starred cards. The output will prefix questions with `Q: ` and answers with `A: `, dividing cards with `---`.

### Learning a set

``` sh
forne learn <set-file>.json -m <method>
```

The above command can be used to start a new learn session on the given set file (created with `forne new` as above). You'll need to provide the method, which will be checked to see if it matches with what the set was created with (if it doesn't, an error will be returned to prevent data loss). The output of this command will be a question, randomly chosen based on the weights assigned by the learning method, and, after pressing enter, you'll be able to say how you did (the responses to this question are determined by the learning method), and the method will adjust the weights accordingly. By default, Forne will keep on presenting cards until you press `Ctrl+D`, or until all cards have weight 0, signifying that you have learned the set. Alternately, you can add `-c <max-count>` to the end of the above command to stop after you've reviewed a certain number of cards, which can be useful for a daily review or the like.

If you want to target only difficult or starred cards, you can add `-t <difficult|starred>` to the end of the above command.

By default, Forne will save your progress in a learning session every time you review a card, but, if you want to start from scratch, you can add `--reset` to the end of the above command. Be aware that this is irreversible though, and your previous progress will be lost forever!

### Testing yourself on a set

``` sh
forne test <set-file>.json
```

Once you've learned a set, you'll probably want to make sure you know everything, and this is where tests come in: they'll present you with each card only once, starring any you get wrong so you can review those specially later. As with the other commands that work on sets, you can add `-t <difficult|starred>` after the above command to only target a certain type of cards, or `--reset` to abort any progress you might have made during a previous test, as Forne will save everything you do so you can come back to it later by default.

As with the learning system, you can add `-c <max-count>` to the end of this command to cap the number of cards you're asked to review.

Note that you don't need to provide a method for testing, as Forne's testing logic is internal, and very simple: you will be shown each card exactly once, and the ones you get wrong are recorded as starred (you can disable this with `--no-star`).

One complication of the test system is that, if you get a card right, and it was previously starred, it will be unstarred immediately, which may mean you lose track of the cards you had previously starred. If you're doing a final review before going into a test, this could be a problem! You can add `--no-unstar` to the above command if you want to disable this behaviour. If you don't want Forne to star or unstar cards whatsoever in a test, you can add `--static`.

## Adapters

The first hurdle to using Forne is importing your set into it. Forne accepts a list of question/answer pairs, but this doesn't mean it can't be used for more exotic use-cases, like a three-language set. Because Forne lets you write your own importing logic, you can very easily take something like a three-way term and turn it into six separate cards (each one going to each other each way) trivially. This also allows things like cloze terms to be supported easily, and in a way that works for you. Forne provides a very simple mechanism to display terms and help you learn them: you control exactly how they're created.

Adapters are written in [Rhai](https://rhai.rs), a simple Rust-like scripting language, and they're pretty easy to write! If you've never done any programming before, you might want to enlist the help of ChatGPT, armed with our [examples of common adapters](https://github.com/arctic-hen7/forne/tree/main/common_adapters), otherwise, go crazy! All adapters are written as simple scripts, which will be have a constant string `SOURCE`, the contents of the given source file, available, and they are expected to return an array of question/answer pairs (e.g. `[["foo", "bar"], ["q", "a"]]`). Most of the time, you can do this with a regular expression, and Forne furnishes you with several utilities for working with regexps:

- `is_match(regexp, text) -> bool`
- `matches(regexp, text) -> Array`
- `captures(regexp, text) -> Array` (this is an array of arrays, where each sub-array is a series of *captures* that the regexp found; index 0 in each one is the full text of the match)
- `replace_one(regexp, replacement, text) -> string`
- `replace_all(regexp, replacement, text) -> string`

There is also a helper function for simple cases that lets you plug in a regular expression with capture groups for the question and answer, the indices of which you provide, and they will be returned. If you don't need to do any further processing, your entire adapter script could be something like this:

```rhai
return regexp_to_pairs(`my-regexp-here`, 1, 2, SOURCE);
```

Here, `1, 2` means the first capture group contains the question, and the second contains the answer. `0` would be the entire match. Note that we put the regular expression in backticks to avoid any escape characters.

We recommend <https://regex101.com> for testing your regular expressions, and non-technical (and technical!) users should be aware that ChatGPT is unreasonably good at producing regular expressions, and even at creating questions from your notes!

For further documentation about the Rhai language, you can refer to the [Rhai book](https://rhai.rs/book), in particular the section on [string manipulation](https://rhai.rs/book/ref/string-fn.html). And, if you need any help writing your own adapter, don't hesitate to open a [new discussion](https://github.com/arctic-hen7/forne/discussions/new/choose) and ask us, we'll be happy to give you a hand!

## Methods

Learning methods are similar to adapters in a lot of ways, except that Forne has several inbuilt, and you can use these by name (e.g. `-m speed-v1`). However, if you want to write your own, to customise your learning process to be more suitable to you, you easily can. First off, you might want to tweak an existing method more than you want to write your own, and you can find the source code for all the inbuilt methods [here](https://github.com/arctic-hen7/forne/tree/main/src/methods).

Method scripts are a little more complicated than adapter scripts, as they need to have a few key elements for Forne to understand them:

1. A `const RESPONSES` array at the start. This should contain all the permissible responses the user can make to a card. For example, the `speed-v1` method uses `const RESPONSES = ["y", "n"];`, meaning the user can either say `y` or `n` when they are told the right answer to a card. Your own methods may define as many responses as they want, and the user will be prompted about which one they wish to choose.
2. A function `get_weight(data, difficult) -> f64`. This function takes in the custom method data (which can be literally whatever the heck you want) and whether or not the card in question is currently marked as difficult, and asks you to return a weight for it, which should be a floating-point (i.e. decimal) number. The probability that any one card will be selected is then this weight divided by the sum of all card weights.
3. A function `adjust_card(response, data, difficult) -> [..., bool]`, which takes in the user's response to a card (guaranteed to be one of the ones you defined in `const RESPONSES`), the card's data, and whether or not it is marked as difficult. It should return the new data (this is where you update the properties that you use to determine a card's weight) and whether or not the card should now be marked as difficult. Note that the meaning of 'difficult' is entirely method-dependent, and it is simply one of the ways Forne lets users see how they're doing with their sets.
4. A function `get_default_metadata() -> ...`, which should return the default values you want to use for a card's `data`.

As an example to help you understand all this a bit better, here's a very naive learning method with heavy commenting:

```rhai
const RESPONSES = ["y", "n"];

fn get_weight(data, difficult) {
   return data.weight;
}
fn adjust_card(res, data, difficult) {
   if res == "y" {
       data.weight -= 0.5;
   } else {
       data.weight += 0.5;
   }

   return [data, false];
}
fn get_default_metadata() {
   return #{ weight: 1.0 };
}
```

This method stores an object with one property, `weight`, for each card, which is `1.0` by default, incrementing it by `0.5` every time the user gets the card wrong, or decrementing it by `0.5` if the user gets it right. This method will never mark a card as difficult. The biggest 'gotcha' when using Rhai is usually the need for a hash sign (`#`) before writing out an object!

As with custom adapters, writing your own learning method can be challenging, and you're more than welcome to open [a discussion](https://github.com/arctic-hen7/forne/discussions/new/choose) and ask us any questions you might have, and we'll be happy to help!

*A final note: plugging this section of the readme into ChatGPT and asking it to write your learning method for you will generally produce workable results, although it doesn't understand Forne perfectly, so you might need to make some minor adjustments. Feel free to ask us in a discussion if you have any questions!*
 
### Contributing custom methods

Forne's mission is to make learning content the easy part of learning, and this requires supporting as many learning methods out of the box as possible. Eventually, people should be able to get a custom adapter off the internet in a jiffy, and they should never need to touch a learning method unless they want to tweak it, because there should be an ample library of alternatives within Forne. To this end, if you've tweaked an inbuilt learning algorithm in a useful way, or if you've written your own from scratch, please submit it through a [pull request](https://github.com/arctic-hen7/forne/pulls) for inclusion in Forne! Provided it works and is far enough from one of the existing inbuilt methods, we're pretty likely to accept it!

Also, if you happen to be researching the science of learning, we'd love to hear from you, and we'd love to help implement your research in practical terms through Forne! Feel free to email the maintainer, [`arctic-hen7`](https://github.com/arctic-hen7) (you'll need to be logged into GitHub to see the email address), to discuss further!

## License and Disclaimer

See [`LICENSE`](./LICENSE).
