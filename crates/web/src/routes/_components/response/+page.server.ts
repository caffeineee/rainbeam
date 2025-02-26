import type { PageServerLoad } from "./$types";
import { type Option, Some, None } from "$lib/classes/Option";
import type { Profile } from "$lib/bindings/Profile";

export let csr = false;

import { langs } from "$lib/lang";
import * as db from "$lib/db";
import type { Serialized } from "$lib/proc/tserde";

export const load: PageServerLoad = async ({
    cookies,
    url,
    request
}): Promise<any> => {
    const token = cookies.get("__Secure-Token");
    const lang = cookies.get("net.rainbeam.langs.choice");

    // build query params map
    let query: Serialized = {};

    for (const param of Array.from(url.searchParams.entries())) {
        query[param[0]] = param[1];
    }

    // fetch page data
    const data = await db.api.get_root(
        {
            route: `_app/${url.pathname.slice(2)}.html${url.search}`,
            version: ""
        },
        {
            body: null,
            headers: request.headers
        }
    );

    // get data
    let data_ = data.status === 200 ? await data.json() : { success: false };

    if (!data_.success) {
        data_.payload = {
            response: undefined,
            anonymous_avatar: "",
            anonymous_username: "",
            is_pinned: false,
            is_powerful: false,
            is_helper: false,
            show_pin_button: false
        };
    }

    // return
    return {
        user: token
            ? Some(
                  (await db.get_profile_from_token(token)).payload as Profile
              ).serialize()
            : (None as Option<Profile>).serialize(),
        lang: langs[lang || "net.rainbeam.langs:en-US"].data,
        config: {
            name: db.config.name,
            description: db.config.description,
            host: db.config.host,
            captcha: {
                site_key: db.config.captcha.site_key
            }
        },
        data: data_.payload,
        query
    };
};
