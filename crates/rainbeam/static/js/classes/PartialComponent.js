class PartialComponent extends HTMLElement {
    static observedAttributes = ["src", "uses"];
    loaded;

    constructor() {
        const self = super();
        self.innerHTML = '<div class="spinner">🐱</div>';
    }

    attributeChangedCallback(name, _, value) {
        switch (name) {
            case "src":
                this.loaded = false;
                fetch(value)
                    .then((res) => res.text())
                    .then((res) => {
                        this.innerHTML = res;

                        if (globalThis[`lib:${value}`]) {
                            // load finished
                            globalThis[`lib:${value}`]();
                        }

                        this.loaded = true;

                        setTimeout(() => {
                            if (!this.getAttribute("uses")) {
                                return;
                            }

                            for (const hook of this.getAttribute("uses").split(
                                ",",
                            )) {
                                trigger(hook);
                            }
                        }, 15);
                    })
                    .catch((err) => {
                        this.innerHTML =
                            "<span>Could not display component.</span>";
                        console.error(err);
                    });

                break;

            case "uses":
                break;

            default:
                return;
        }
    }
}

customElements.define("include-partial", PartialComponent);
define("PartialComponent", PartialComponent);
