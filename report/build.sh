#!/bin/bash

set -e # Exit immediately if a command exits with a non-zero status

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "${SCRIPT_DIR}"

OUTPUT_DIR="out"
MAIN_FILE="index"
TEX_FILE="${MAIN_FILE}.tex"

run_pdflatex() {
    echo "Running pdflatex..."
    pdflatex \
        -synctex=1 \
        -interaction=nonstopmode \
        -file-line-error \
        -recorder \
        -output-directory="${OUTPUT_DIR}" \
        "${TEX_FILE}"
}

run_biber() {
    echo "Running biber..."
    biber "${OUTPUT_DIR}/${MAIN_FILE}"
}

clean_aux() {
    echo "Cleaning auxiliary files..."
    rm -rf "${OUTPUT_DIR}"/*.aux \
           "${OUTPUT_DIR}"/*.bbl \
           "${OUTPUT_DIR}"/*.bcf \
           "${OUTPUT_DIR}"/*.blg \
           "${OUTPUT_DIR}"/*.log \
           "${OUTPUT_DIR}"/*.out \
           "${OUTPUT_DIR}"/*.run.xml
}

mkdir -p "${OUTPUT_DIR}"

# Run each step and capture its exit status
echo "Starting LaTeX build process..."

if ! run_pdflatex; then
    echo "Error during first pdflatex run"
    cat "${OUTPUT_DIR}/${MAIN_FILE}.log"
    exit 1
fi

if ! run_biber; then
    echo "Error during biber run"
    cat "${OUTPUT_DIR}/${MAIN_FILE}.blg"
    exit 1
fi

if ! run_pdflatex; then
    echo "Error during second pdflatex run"
    cat "${OUTPUT_DIR}/${MAIN_FILE}.log"
    exit 1
fi

if ! run_pdflatex; then
    echo "Error during third pdflatex run"
    cat "${OUTPUT_DIR}/${MAIN_FILE}.log"
    exit 1
fi

echo "Build completed successfully!"
clean_aux
git add "${OUTPUT_DIR}/index.pdf"