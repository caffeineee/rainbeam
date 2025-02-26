<script lang="ts">
    import { onMount } from "svelte";

    import { active_page } from "$lib/stores";
    import { BadgePlus, Check, ChevronRight, Ellipsis, PaintBucket, Pen, X } from "lucide-svelte";
    import { Option } from "$lib/classes/Option";
    active_page.set("settings.profile");

    const { data } = $props();
    const lang = data.lang;
    const user = Option.from(data.user).unwrap();

    onMount(() => {
        setTimeout(() => {
            use("codemirror", (codemirror: any) => {
                codemirror.create_editor(
                    document.getElementById("css_editor"),
                    (document.getElementById("sparkler:custom_css") as any).value,
                    "/* enter custom CSS */",
                    "css_editor_",
                    "css"
                );
            });
        }, 500);
    });
</script>

<div class="flex flex-col gap-4" data-tab="settings">
    <h4 class="title">
        {lang["settings:profile_options.html:title.about_me"]}
    </h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:display_name">
            {lang["settings:profile_options.html:label.display_name"]}
        </label>

        <input
            name="sparkler:display_name"
            id="sparkler:display_name"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:display_name", event.target.value);
            }}
        />

        <p class="fade">
            Your display name is shown about your username on your profile and can be used to have a name that is not
            bound by character limits!
        </p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:biography" class="flex gap-2 items-center">
            {lang["settings:profile_options.html:label.biography"]}
        </label>

        <textarea
            name="sparkler:biography"
            id="sparkler:biography"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:biography", event.target.value);
            }}
        ></textarea>

        <p class="fade">A short description about you. Supports Markdown.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:sidebar" class="flex gap-2 items-center">
            {lang["settings:profile_options.html:label.sidebar"]}
        </label>

        <textarea
            name="sparkler:sidebar"
            id="sparkler:sidebar"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:sidebar", event.target.value);
            }}
        ></textarea>

        <p class="fade">
            A sidebar shown under your profile card on your profile. Use it to display extra information separate from
            your biography. Supports Markdown.
        </p>
    </div>

    <h4 class="title">Images</h4>
    <div class="flex flex-col gap-1">
        <label for="avatar_file">
            {lang["settings:profile_options.html:label.avatar"]}
        </label>

        <form
            class="flex gap-2 flex-row items-center"
            method="post"
            enctype="multipart/form-data"
            action="/api/v0/auth/me/upload_avatar"
        >
            <input
                id="avatar_file"
                name="file"
                type="file"
                accept="image/png,image/jpeg,image/avif,image/webp"
                class="w-content"
            />

            <button class="primary"><Check class="icon" /></button>
        </form>

        <p class="fade">Uploaded files must be less than 8 MB and will be converted to AVIF.</p>

        <details class="w-full">
            <summary class="flex items-center gap-2">
                <Ellipsis class="icon" />
                <span>
                    {lang["settings:profile_options.html:label.advanced"]}
                </span>
            </summary>

            <div class="card secondary w-full flex flex-col gap-1">
                <label for="sparkler:avatar_url">
                    {lang["settings:profile_options.html:label.avatar_url"]}
                </label>

                <input
                    type="url"
                    name="sparkler:avatar_url"
                    id="sparkler:avatar_url"
                    onchange={(event: any) => {
                        (globalThis as any).update_kv("sparkler:avatar_url", event.target.value);
                    }}
                />

                <p class="fade">Put a link to your avatar above.</p>
            </div>
        </details>
    </div>

    <div class="flex flex-col gap-1 title">
        <label for="banner_file">
            {lang["settings:profile_options.html:label.banner"]}
        </label>

        <form
            class="flex gap-2 flex-row items-center"
            method="post"
            enctype="multipart/form-data"
            action="/api/v0/auth/me/upload_banner"
        >
            <input
                id="banner_file"
                name="file"
                type="file"
                accept="image/png,image/jpeg,image/avif,image/webp"
                class="w-content"
            />

            <button class="primary"><Check class="icon" /></button>
        </form>

        <p class="fade">Uploaded files must be less than 8 MB and will be converted to AVIF.</p>

        <details class="w-full">
            <summary class="flex items-center gap-2">
                <Ellipsis class="icon" />
                <span>
                    {lang["settings:profile_options.html:label.advanced"]}
                </span>
            </summary>

            <div class="card secondary w-full flex flex-col gap-1">
                <label for="sparkler:banner_url">
                    {lang["settings:profile_options.html:label.banner_url"]}
                </label>

                <input
                    type="url"
                    name="sparkler:banner_url"
                    id="sparkler:banner_url"
                    onchange={(event: any) => {
                        (globalThis as any).update_kv("sparkler:banner_url", event.target.value);
                    }}
                />

                <p class="fade">Put a link to your banner above. This is the big thing at the top of your profile.</p>
            </div>
        </details>
    </div>

    <div class="flex flex-col gap-1 title">
        <label for="sparkler:banner_fit">
            {lang["settings:profile_options.html:label.banner_fit"]}
        </label>

        <select
            name="sparkler:banner_fit"
            id="sparkler:banner_fit"
            onchange={(event: any) => {
                (globalThis as any).update_kv(
                    "sparkler:banner_fit",
                    event.target.options[event.target.selectedIndex].value
                );
            }}
        >
            <option value="cover" id="cover">Cover (default)</option>
            <option value="fill" id="fill">Fill</option>
            <option value="contain" id="contain">Contain</option>
        </select>

        <p class="fade">The way your banner image is fit onto the image element.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:desktop_tl_logo">
            {lang["settings:profile_options.html:label.desktop_top_left_logo"]}
        </label>

        <input
            type="url"
            name="sparkler:desktop_tl_logo"
            id="sparkler:desktop_tl_logo"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:desktop_tl_logo", event.target.value);
            }}
        />

        <p class="fade">The image located at the top left of Neospring on desktop.</p>
    </div>

    <h4 class="title">
        {lang["settings:profile_options.html:title.greetings"]}
    </h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:motivational_header" class="flex gap-2 items-center">
            {lang["settings:profile_options.html:label.motivational_header"]}
        </label>

        <input
            name="sparkler:motivational_header"
            id="sparkler:motivational_header"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:motivational_header", event.target.value);
            }}
        />

        <p class="fade">Motivate people to ask you a question! Shown just above the question input.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:warning" class="flex gap-2 items-center">
            {lang["settings:profile_options.html:label.profile_warning"]}
        </label>

        <textarea
            name="sparkler:warning"
            id="sparkler:warning"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:warning", event.target.value);
            }}
        ></textarea>

        <p class="fade">
            This will show a warning on your profile when people visit it. People will be given the option to continue
            to your profile or go the Sparkler homepage. Set this to nothing to disable it!
        </p>
    </div>

    <h4 class="title">
        {lang["settings:profile_options.html:title.anonymous"]}
    </h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:anonymous_username">
            {lang["settings:profile_options.html:label.anonymous_username"]}
        </label>

        <input
            name="sparkler:anonymous_username"
            id="sparkler:anonymous_username"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:anonymous_username", event.target.value);
            }}
        />

        <p class="fade">This name will be shown instead of "anonymous" in the username slot of anonymous questions.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:anonymous_avatar">
            {lang["settings:profile_options.html:label.anonymous_avatar_url"]}
        </label>

        <input
            type="url"
            name="sparkler:anonymous_avatar"
            id="sparkler:anonymous_avatar"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:anonymous_avatar", event.target.value);
            }}
        />

        <p class="fade">This avatar will be shown instead of the default avatar on anonymous questions.</p>
    </div>

    <h4 class="title">{lang["settings:profile_options.html:title.feed"]}</h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:pinned">
            {lang["settings:profile_options.html:label.pinned"]}
        </label>

        <textarea
            name="sparkler:pinned"
            id="sparkler:pinned"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:pinned", event.target.value);
            }}
        ></textarea>

        <p class="fade">
            Put the ID of the responses you want to pin there. Note this will also pin the question! You can also do
            this by clicking "Pin" on any of your responses. (Responses should be separated by a comma, with a trailing
            comma at the end!)
        </p>
    </div>

    <h4 class="title">Theming</h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:profile_theme">Profile theme</label>

        <select
            name="sparkler:profile_theme"
            id="sparkler:profile_theme"
            onchange={(event: any) => {
                (globalThis as any).update_kv(
                    "sparkler:profile_theme",
                    event.target.options[event.target.selectedIndex].value
                );
            }}
        >
            <option value="">User-choice</option>
            <option value="light" id="light">
                {lang["settings:text.light"]}
            </option>
            <option value="dark" id="dark">Dark</option>
            <option value="dark dim" id="dark dim">
                {lang["settings:text.dim"]}
            </option>
        </select>

        <p class="fade">This is the base theme used on your profile.</p>
    </div>

    <h4 class="title">Layout</h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:layout">Profile layout</label>

        <select
            name="sparkler:layout"
            id="sparkler:layout"
            onchange={(event: any) => {
                (globalThis as any).update_kv(
                    "sparkler:layout",
                    event.target.options[event.target.selectedIndex].value
                );
            }}
        >
            <option value="0" id="0">Sidebar on left (default)</option>
            <option value="1" id="1">Sidebar on right</option>
        </select>

        <p class="fade">The layout used on your profile.</p>
    </div>

    <h4 class="title">Colors</h4>

    <div class="markdown-alert-important flex flex-col gap-2" style="align-items: flex-start">
        <b>Theming best practices:</b>

        <ol>
            <li>Make sure your theme is easily readable and accessible</li>
            <li>Make sure your theme follows a coherent color scheme</li>
            <li>
                Make sure your theme isn't too bright or too dark, as many users could be sensitive to extreme colors
            </li>
        </ol>
    </div>

    <div class="flex flex-col gap-2">
        <a href="/market/new?type=UserTheme" class="card w-full flex justify-between items-center gap-2">
            <div class="flex gap-2 items-center">
                <BadgePlus class="icon" />
                <span> {lang["settings:coins.html:text.save_theme"]} </span>
            </div>

            <button class="primary icon-only small">
                <ChevronRight class="icon" />
            </button>
        </a>

        <a href="/market?creator={user.id}&type=UserTheme" class="card w-full flex justify-between items-center gap-2">
            <div class="flex gap-2 items-center">
                <PaintBucket class="icon" />
                <span> {lang["settings:coins.html:text.my_themes"]} </span>
            </div>

            <button class="primary icon-only small">
                <ChevronRight class="icon" />
            </button>
        </a>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_surface">Surface</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_surface", event.target.value);
                }}
            />

            <input
                name="sparkler:color_surface"
                id="sparkler:color_surface"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_surface", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Page background.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_lowered">Lowered</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_lowered", event.target.value);
                }}
            />

            <input
                name="sparkler:color_lowered"
                id="sparkler:color_lowered"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_lowered", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">
            Stuff that is darker than the page background. (questions, motivational header, buttons, textarea, etc.)
        </p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_super_lowered">Super lowered</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_super_lowered", event.target.value);
                }}
            />

            <input
                name="sparkler:color_super_lowered"
                id="sparkler:color_super_lowered"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_super_lowered", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Even darker than that. (button hover, etc.)</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_raised">Raised</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_raised", event.target.value);
                }}
            />

            <input
                name="sparkler:color_raised"
                id="sparkler:color_raised"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_raised", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">
            Stuff that is lighter than the page background. (profile card, pill menu button hover, responses, etc.)
        </p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_super_raised">Super raised</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_super_raised", event.target.value);
                }}
            />

            <input
                name="sparkler:color_super_raised"
                id="sparkler:color_super_raised"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_super_raised", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Even lighter than that. (pill menu buttons, etc.)</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_text">Text</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_text", event.target.value);
                }}
            />

            <input
                name="sparkler:color_text"
                id="sparkler:color_text"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_text", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Text color used everywhere.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_text_raised">Text raised</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_text_raised", event.target.value);
                }}
            />

            <input
                name="sparkler:color_text_raised"
                id="sparkler:color_text_raised"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_text_raised", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Text color used on raised elements (profile card, responses, etc.) everywhere.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_text_lowered">Text lowered</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_text_lowered", event.target.value);
                }}
            />

            <input
                name="sparkler:color_text_lowered"
                id="sparkler:color_text_lowered"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_text_lowered", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Text color used on lowered elements (motivation header, questions, etc.) everywhere.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_link">Link color</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_link", event.target.value);
                }}
            />

            <input
                name="sparkler:color_link"
                id="sparkler:color_link"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_link", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Text color used on links.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_primary">Primary</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_primary", event.target.value);
                }}
            />

            <input
                name="sparkler:color_primary"
                id="sparkler:color_primary"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_primary", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">That primary pink-ish color.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_primary_lowered">Primary lowered</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_primary_lowered", event.target.value);
                }}
            />

            <input
                name="sparkler:color_primary_lowered"
                id="sparkler:color_primary_lowered"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_primary_lowered", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">That primary pink-ish color when you hover over it (buttons).</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_primary_raised">Primary raised</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_primary_raised", event.target.value);
                }}
            />

            <input
                name="sparkler:color_primary_raised"
                id="sparkler:color_primary_raised"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_primary_raised", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">A lightly lighter version of your primary color.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_text_primary">Text primary</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_text_primary", event.target.value);
                }}
            />

            <input
                name="sparkler:color_text_primary"
                id="sparkler:color_text_primary"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_text_primary", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Text color used on elements using the primary color as a background.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="sparkler:color_shadow">Shadow</label>

        <div class="flex gap-2 flex-row">
            <input
                type="color"
                onchange={(event: any) => {
                    (globalThis as any).link_color("sparkler:color_shadow", event.target.value);
                }}
            />

            <input
                name="sparkler:color_shadow"
                id="sparkler:color_shadow"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:color_shadow", event.target.value);
                }}
                class="w-full"
            />
        </div>

        <p class="fade">Color used for shadows.</p>
    </div>

    <h4 class="title">Advanced</h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:custom_css">Custom CSS</label>

        <textarea
            name="sparkler:custom_css"
            id="sparkler:custom_css"
            autocomplete="off"
            autocapitalize="off"
            spellcheck={false}
            class="hidden"
        ></textarea>

        <a href="#/css" class="button">
            <Pen class="icon" />
            <span>{lang["general:action.edit"]}</span>
        </a>

        <p class="fade">Custom profile CSS that accepts more than just colors.</p>
    </div>

    <div class="flex flex-col gap-1">
        <label for="rainbeam:market_theme_template">Theme base</label>

        <button
            onclick={() => {
                (globalThis as any).update_kv("rainbeam:market_theme_template", "");
            }}
        >
            {lang["general:action.clear"]}
        </button>

        <p class="fade">Pressing "Clear" will reset the theme base applied from market themes.</p>
    </div>
</div>

<div class="hidden flex flex-col gap-2" data-tab="css">
    <script src="https://unpkg.com/codemirror@5.39.2/lib/codemirror.js"></script>
    <script src="https://unpkg.com/codemirror@5.39.2/addon/display/placeholder.js"></script>
    <script src="https://unpkg.com/codemirror@5.39.2/mode/css/css.js"></script>

    <link rel="stylesheet" href="https://unpkg.com/codemirror@5.39.2/lib/codemirror.css" />

    <div class="flex w-full gap-2 justify-between">
        <b>Custom CSS</b>

        <div class="flex gap-2">
            <button
                class="primary bold"
                onclick={() => {
                    (globalThis as any).update_kv("sparkler:custom_css", (globalThis as any).css_editor_.getValue());
                }}
            >
                <Check class="icon" />
                <span>{lang["general:action.save"]}</span>
            </button>

            <a class="button red bold" href="#/settings">
                <X class="icon" />
                <span>{lang["general:action.close"]}</span>
            </a>
        </div>
    </div>

    <div id="css_editor" class="css_editor card secondary"></div>

    <style>
        .CodeMirror {
            height: max-content !important;
        }

        .CodeMirror-line {
            font-family: ui-monospace, monospace !important;
        }
    </style>
</div>
