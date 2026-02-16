// @ts-check
function init() {
    console.log("hello world")

    document.querySelectorAll("[data-counter]")
    .forEach((el) => {
        const output = el.querySelector("[data-counter-output]"),
        increment = el.querySelector("[data-counter-increment]");

        increment.addEventListener("click", e => {
            console.log("clicked");
            output.textContent++; }
            );
    })
}

document.addEventListener("DOMContentLoaded", init);