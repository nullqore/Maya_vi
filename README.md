# Maya VI - Sitemap Generator & SEO Audit Tool

A super-fast and lightweight Rust-based tool for generating sitemaps from a list of URLs and performing SEO audits.

## Features

*   **Sitemap Generation**: Load a list of URLs from a file (`test_urls.txt`) to generate a `sitemap.txt`.
*   **Proxy Support**: Option to route requests through a proxy for versatile network testing.
*   **Request/Response Viewer**: Inspect HTTP requests and responses for in-depth analysis and debugging.
*   **Data Management**: Easily delete and save your data.
*   **High Performance**: Built in Rust for a speedy and lightweight experience.

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
2.  The application will automatically load URLs from `test_urls.txt`.
3.  Use the provided options to render through a proxy, view requests and responses, and manage your data.
4.  A `sitemap.txt` file will be generated based on your list of URLs.

## Dependencies

This project uses the following main dependencies:

*   [eframe](https://github.com/emilk/egui/tree/master/crates/eframe): For the GUI framework.
*   [reqwest](https://crates.io/crates/reqwest): For making HTTP requests.
*   [url](https://crates.io/crates/url): For URL parsing.
*   [syntect](https://crates.io/crates/syntect): For syntax highlighting.
*   [sled](https://crates.io/crates/sled): For the embedded database.

## Contributing

Contributions are welcome! Please feel free to submit a pull request or open an issue.

## License

This project is not yet licensed.
