# Maya VI - Sitemap Generator

A powerful and efficient sitemap generator built with Rust. Maya VI crawls a given URL and generates a sitemap, which can be used for SEO purposes.

## Features

*   **Web Crawling**: Traverses a website by following links to discover all pages.
*   **Sitemap Generation**: Creates a `sitemap.txt` file containing all discovered URLs.
*   **Cross-platform**: Built with `eframe` for a native GUI experience on Windows, macOS, and Linux.
*   **Syntax Highlighting**: Displays file contents with syntax highlighting.

## Installation

1.  **Clone the repository:**
    ```bash
    git clone <repository-url>
    cd maya_vi
    ```

2.  **Build the project:**
    ```bash
    cargo build --release
    ```

3.  **Run the application:**
    ```bash
    cargo run --release
    ```

## Usage

1.  Launch the application.
2.  Enter the base URL you want to crawl in the input field.
3.  Click the "Crawl" button.
4.  The application will start crawling the website and will display the discovered URLs in real-time.
5.  Once the crawl is complete, a `sitemap.txt` file will be created in the project's root directory.

## Dependencies

This project uses the following main dependencies:

*   [eframe](https://github.com/emilk/egui/tree/master/crates/eframe): For the GUI framework.
*   [reqwest](https://crates.io/crates/reqwest): For making HTTP requests.
*   [url](https://crates.io/crates/url): For URL parsing.
*   [syntect](https://crates.io/crates/syntect): For syntax highlighting.
*   [sled](https://crates.io/crates/sled): For the embedded database.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.
