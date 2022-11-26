<script setup lang="ts">
    import { ref, onMounted } from "vue";

    import { apiInitialize, apiValidateSession } from "../api_js/serverAPI";
    import { jump_to_login, jump_to_after_login } from "./Helpers";

    import Logout from "./Logout.vue";
    import HelpLink from "./HelpLink.vue";
    import Spinner from "./Spinner.vue";

    const app_version = __APP_VERSION__;
    
    const show_spinner = ref(true);

    const props = defineProps({
        // Set this true if the page should only be accessible when logged in
        // Don't set this if the page should only be accessible when NOT logged in
        loggedIn: Boolean,
        showTitle: Boolean,
    });

    apiInitialize();

    onMounted( () => {
        show_spinner.value = true;
        if( props.loggedIn ) {
            // Page requires user to be logged in
            apiValidateSession()
                .then( () => { 
                    /* Validation succeeded, so allow page */
                    show_spinner.value = false;
                } )
                .catch( (_error: any) => {
                    /* User not logged in... */
                    jump_to_login();
                } );
        } else {
            // Page requires user to not be logged in
            apiValidateSession()
                .then( () => {
                    /* User is logged in... */
                    jump_to_after_login();
                } )
                .catch( (_error: any) => {
                    /* Validation failed, so allow page */
                    show_spinner.value = false;
                } );
        }
    });
</script>

<template>
    <div :app-ver="app_version">
        <div id="header_area">
            <div id="hdr_name" v-if="showTitle">Running Stream</div>
            <div id="mgmt_btns">
                <HelpLink v-if="showTitle"/>
                <Logout v-if="loggedIn"/>
            </div>
        </div>
        <div id="screen_area">
            <slot />
        </div>
        <Spinner :display="show_spinner" />
    </div>
</template>

<style>
    :root {
        --color-scarlet: #b85042;
        --color-dkscarlet: #8f1014;
        --color-dkscarlet-rgb: 143, 16, 20;
        --color-ltolive: #e7e8d1;
        --color-teal: #a7beae;
        --color-ltteal: #dbeee1;
    }

    /* Quick reset */
    *, *::before, *::after {
        margin: 0;
        box-sizing: border-box;
    }

    .bigger_text {
        font-weight: 700;
        font-size: 1em;
        padding: 10px 15px;
    }

    body {
        background: var(--color-ltolive);
        color: var(--color-dkscarlet);
        margin: 0;
        padding: 0;
        width: 100%;
        font-family: serif;
    }

    input[type="text"], input[type="password"] {
        background-color: var(--color-ltolive);
        border: 1px var(--color-scarlet) solid;
        border-radius: 10px;
        color: var(--color-dkscarlet);
        padding: 5px;
        margin: 5px;
        max-width: 100%;
    }

    input[type="submit"], input[type="button"], .input_button {
        background-color: var(--color-scarlet);
        border: 1px var(--color-ltolive) solid;
        border-radius: 20px;
        color: var(--color-ltolive);
        margin: 5px;
        font-family: sans-serif;
        text-decoration: none;
        text-shadow: none;
        cursor: pointer;
    }

    input[type="submit"]:active, input[type="button"]:active, .input_button:active {
        box-shadow: inset 5px 5px var(--color-dkscarlet);
    }

    a {
        color: inherit;
    }

    h1 {
        font-size: 5em;
        font-weight: bold;
    }

    h2 {
        font-size: 2em;
        font-weight: bold;
    }

    h3 {
        font-size: 1.5em;
        font-weight: bold;
    }
    h4 {
        font-size: 1em;
        font-weight: bold;
    }
    h1, h2 {
        margin-block-start: .83em;
        margin-block-end: .83em;
    }
    h3, p {
        margin-block-start: 1em;
        margin-block-end: 1em;
    }
    h4 {
        margin-block-start: 1.33em;
        margin-block-end: 1.33em;
    }
    h2, h3, h4, p {
        margin-inline-start: 0;
        margin-inline-end: 0;
    }

    @media (max-width: 700px) {
        h1 {
            font-size: 3em;
        }
        h3 {
            font-size: 1.5em;
        }
    }
</style>

<style scoped>
    #header_area {
        position: fixed;
        min-height: 2em;
        height: -moz-fit-content;
        height: fit-content;
        width: 100%;
        left: 0px;
        top: 0px;
        background-color: var(--color-scarlet);
        z-index: 1;
        display: flex;
    }

    #hdr_name {
        font-size: 1.5em;
        font-weight: bold;
        color: var(--color-ltolive);
    }

    #mgmt_btns {
        display: flex;
        margin-left: auto;
    }

    #mgmt_btns > div, #mgmt_btns > a {
        background-size: cover;
        height: 2em;
        width: 2em;
        cursor: pointer;
        text-decoration: none;
        margin-right: .5em;
    }

    #screen_area {
        margin-bottom: 0;
        margin-top: 2em;
    }
</style>
