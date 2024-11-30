#!/bin/bash

set -e # Exit immediately if a command exits with a non-zero status

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "${SCRIPT_DIR}"

OUTPUT_DIR="out"
MAIN_FILE="index"
TEX_FILE="${MAIN_FILE}.tex"

run_pdflatex() {
    echo "Running pdflatex..."
    PDFLATEX_OUTPUT=$(
        pdflatex \
        -synctex=1 \
        -interaction=nonstopmode \
        -file-line-error \
        -recorder \
        -output-directory="${OUTPUT_DIR}" \
        "${TEX_FILE}"
    )
    if [ $? -ne 0 ]; then
        echo "Error in pdflatex:"
        echo $PDFLATEX_OUTPUT | grep "LaTeX Error" -A 10
        exit 1
    fi
}

run_biber() {
    echo "Running biber..."
    biber "${OUTPUT_DIR}/${MAIN_FILE}" -q
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
run_pdflatex # First pdflatex run
run_biber # Run biber to process bibliography
run_pdflatex # Second pdflatex run to incorporate bibliography
run_pdflatex # Third pdflatex run to resolve references
echo "Build completed successfully!"
clean_aux
git add "${OUTPUT_DIR}/index.pdf"