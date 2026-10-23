# rust-course

A complete, self-paced Rust curriculum in Markdown: 36 lessons across Beginner, Intermediate and Advanced, each level ending in a capstone project.

**Read it online:** https://hunterghall.github.io/rust-course/

The buildable source of the three capstones lives in [`capstones/`](capstones) (task tracker, Markdown link checker, parallel file indexer) and is built, tested and linted by CI on Linux, macOS and Windows. CI also compile-checks every Rust code block in the lessons (`tools/check_examples.py`).

To preview the site locally:

```bash
pip install mkdocs-material
mkdocs serve
```

To check the lesson code yourself (needs a Rust toolchain):

```bash
python tools/check_examples.py          # compile every ```rust block in docs/
python tools/sync_capstones.py --check  # confirm lessons show the current capstone source
bash capstones/ci.sh                    # build and test the three capstones
```
