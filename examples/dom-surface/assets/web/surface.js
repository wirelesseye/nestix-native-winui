const status = document.querySelector("#template-script-status");

if (status) {
    status.textContent = "Template script loaded";
}

class NestixCounterAction extends HTMLElement {
    #activate = () => {
        this.dispatchEvent(new CustomEvent("increment"));
    };

    #onKeyDown = (event) => {
        if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            this.#activate();
        }
    };

    connectedCallback() {
        this.setAttribute("role", "button");
        this.tabIndex = 0;
        this.addEventListener("click", this.#activate);
        this.addEventListener("keydown", this.#onKeyDown);
    }

    disconnectedCallback() {
        this.removeEventListener("click", this.#activate);
        this.removeEventListener("keydown", this.#onKeyDown);
    }

    set currentCount(value) {
        this.dataset.count = String(value);
    }
}

customElements.define("nestix-counter-action", NestixCounterAction);

document.documentElement.dataset.templateScript = "ready";
