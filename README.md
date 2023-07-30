# Formanator 🤖

Formanator allows you to submit benefit claims to [Forma](https://www.joinforma.com/) from the command line, either one-by-one or in bulk.

## Installation

To install Formanator, just run:

```bash
npm install -g formanator
```

You'll need to be running at least [Node.js](https://nodejs.org/en) v18.

## Usage

### Connecting to your Forma account

To get started, you'll need to connect Formanator to your Forma account. Here's how the process works:

1. Run `formanator login`.
2. At the prompt, enter your email address, then hit Enter.
3. You'll be sent an email with a magic link. Go to your inbox and copy the link to your clipboard.
4. At the prompt, paste your magic link, then hit Enter.
5. You'll be logged in 🥳

To remember your login, Formanator stores a `.formanator.json` file in your home directory with your access token.

### Submitting a single claim

You can quickly submit a single claim from the command line.

1. Figure out what you're planning to claim for.
2. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
3. Get a list of your available benefits by running `formanator benefits`. 
4. Pick the relevant benefit, and then run `formanator categories --benefit <benefit>` to get a list of categories. For example, running `formanator categories --benefit "Learning"` would return the applicable categories for the "Learning" benefit.
5. Submit your claim by running `formanator submit-claim`. You'll need to pass a bunch of arguments:

```bash
formanator submit-claim --benefit "Remote Life" \
                        --category "Cables & Cords" \
                        --amount 2.28 \
                        --merchant Amazon \
                        --description "USB cable" \
                        --purchase-date 2023-01-15 \
                        --receipt-path "USB.pdf"
```

6. Your claim will be submitted.

### Submiting multiple claims

You can submit multiple claims at once by generating a template CSV, filling it in, then submitting the whole CSV.

1. Make sure you're logged in - for more details, see "Connecting to your Forma account" above.
2. Run `formanator generate-template-csv` to generate a CSV template. By default, the template will be saved as `claims.csv`. Optionally, you can specify the `--output-path` argument to choose where to save the template.
3. Update the template, filling in the columns for each of your claims. To get valid `benefit` and `category` values, use the `formanator benefits` and `formanator categories --benefit <benefit>` commandsa documented in "Submitting a single claim" above.
4. Submit your claims by running `formanator submit-claims-from-csv --input-path claims.csv`.
5. Your claims will be submitted. If there are any validation errors with any of the rows, or if anything goes wrong during submission, an error message will be displayed, but the tool will continue submitting other claims.

## Contributing

Changes to this project are verioned using [Semantic Versioning](https://semver.org/) and released to `npm` automatically using [`semantic-release`](https://github.com/semantic-release/semantic-release). 

Commit messages must follow [Angular Commit Message Conventions](https://github.com/angular/angular/blob/master/CONTRIBUTING.md#-commit-message-format) so `semantic-release` knows when to release new versions and what version number to use.