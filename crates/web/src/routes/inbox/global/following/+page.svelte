<script lang="ts">
    import { Option } from "$lib/classes/Option.js";
    import type { Profile } from "$lib/bindings/Profile.js";
    import GlobalQuestion from "$lib/components/GlobalQuestion.svelte";
    import { onMount } from "svelte";
    import { active_page } from "$lib/stores.js";

    const { data } = $props();
    active_page.set("timeline");

    const user = Option.from(data.user);

    const lang = data.lang;
    const page = data.data;
    const config = data.config;
    const questions = page.questions;

    onMount(async () => {
        // partial
        setTimeout(() => {
            trigger("questions:carp");
        }, 100);
    });
</script>

<svelte:head>
    <title>{config.name}</title>
    <meta name="description" content={config.description} />
</svelte:head>

<article>
    <main class="flex flex-col gap-2">
        <div class="pillmenu convertible">
            <a href="/"><span>{lang["timelines:link.timeline"]}</span></a>
            <a href="/inbox/posts"><span>{lang["timelines:link.posts"]}</span></a>
            <a href="/inbox/global" class="active"><span>{lang["timelines:link.global"]}</span></a>
        </div>

        <div class="pillmenu convertible">
            <a href="/inbox/global"><span>{lang["timelines:link.public"]}</span></a>
            <a href="/inbox/global/following" class="active"><span>{lang["timelines:link.following"]}</span></a>
        </div>

        <div id="feed" class="flex flex-col gap-2">
            {#each questions as ques}
                <GlobalQuestion {ques} show_responses={true} profile={user} />
            {/each}
        </div>
    </main>
</article>
