<script lang="ts">
    import { active_page } from "$lib/stores";
    import { Option } from "$lib/classes/Option";
    active_page.set("settings.privacy");

    const { data } = $props();
    const lang = data.lang;
</script>

<div class="flex flex-col gap-4">
    <h4 class="title">
        {lang["settings:privacy_options.html:title.general_privacy"]}
    </h4>
    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:lock_profile"
                id="sparkler:lock_profile"
                onchange={(event: any) => {
                    (globalThis as any).update_kv("sparkler:lock_profile", event.target.checked.toString());
                }}
            />

            <label for="sparkler:lock_profile" class="normal">
                {lang["settings:privacy_options.html:label.disable_questions"]}
            </label>
        </div>

        <p class="fade subtext">Disable the question asking feature for your profile.</p>
    </div>

    <div class="checkbox_container">
        <input
            type="checkbox"
            name="sparkler:disallow_anonymous"
            id="sparkler:disallow_anonymous"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:disallow_anonymous", event.target.checked.toString());
            }}
        />

        <label for="sparkler:disallow_anonymous" class="normal">
            {lang["settings:privacy_options.html:label.disable_anonymous_questions"]}
        </label>
    </div>

    <div class="flex flex-col gap-1">
        <div class="checkbox_container">
            <input
                type="checkbox"
                name="sparkler:disallow_anonymous_comments"
                id="sparkler:disallow_anonymous_comments"
                onchange={(event: any) => {
                    (globalThis as any).update_kv(
                        "sparkler:disallow_anonymous_comments",
                        event.target.checked.toString()
                    );
                }}
            />

            <label for="sparkler:disallow_anonymous_comments" class="normal">
                {lang["settings:privacy_options.html:label.disable_anonymous_comments"]}
            </label>
        </div>
    </div>

    <div class="checkbox_container">
        <input
            type="checkbox"
            name="sparkler:require_account"
            id="sparkler:require_account"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:require_account", event.target.checked.toString());
            }}
        />

        <label for="sparkler:require_account" class="normal">
            {lang["settings:privacy_options.html:label.require_account"]}
        </label>
    </div>

    <div class="checkbox_container">
        <input
            type="checkbox"
            name="sparkler:private_social"
            id="sparkler:private_social"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:private_social", event.target.checked.toString());
            }}
        />

        <label for="sparkler:private_social" class="normal">
            {lang["settings:privacy_options.html:label.keep_socials_private"]}
        </label>
    </div>

    <h4 class="title">
        {lang["settings:privacy_options.html:title.moderation"]}
    </h4>
    <div class="flex flex-col gap-1">
        <label for="sparkler:filter">{lang["settings:privacy_options.html:label.filter"]}</label>

        <textarea
            name="sparkler:filter"
            id="sparkler:filter"
            onchange={(event: any) => {
                (globalThis as any).update_kv("sparkler:filter", event.target.value);
            }}
        ></textarea>

        <p class="fade">
            {lang["settings:privacy_options.html:text.filter"]}
        </p>
    </div>
</div>
