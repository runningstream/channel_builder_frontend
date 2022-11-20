<script setup lang="ts">
    import { ref } from "vue";

    import { apiValidateSession, apiAuthenticate } from "../api_js/serverAPI";
    import { jump_to_after_login } from "./Helpers";

    import Usecase from "./Usecase.vue";
    import Alert from "./Alert.vue";
    import Spinner from "./Spinner.vue";

    const username = ref("");
    const password = ref("");

    const alert_login_succ_val_fail = ref(false);
    const alert_login_fail = ref(false);
    const show_spinner = ref(false);

    function form_submit(_ev: Event) : void {
        show_spinner.value = true;
        apiAuthenticate(username.value, password.value)
            .then( () => {
                apiValidateSession()
                    .then( () => {
                        show_spinner.value = false;
                        jump_to_after_login();
                    } )
                    .catch( (_error: any) => {
                        show_spinner.value = false;
                        alert_login_succ_val_fail.value = true;
                    });
            })
            .catch( (_error: any) => {
                show_spinner.value = false;
                alert_login_fail.value = true;
            });
    }
</script>

<template>
    <section id="intro">
        <div id="intro_text">
            <h1>Running Stream</h1>
            <h2>Your Personal Roku Channel</h2>
            <br/>
            <a href="/signup.html" class="input_button bigger_text">Sign Up</a>
            <a href="#login" class="input_button bigger_text">Sign In</a>
        </div>
    </section>
    <section id="intro2">
        <div>Stream your videos, through your channel.  Build it quickly and easily.</div>
        <div>Typically with a Roku or other streaming device you're limited to streaming content someone else has provided.  With Running Stream you can stream your personal or business videos through your Roku easily.</div>
    </section>
    <section id="usecases">
        <h1>Why?</h1>
        <p>Tap the baby example for a demo...</p>

        <Usecase
            img="img/baby.svg"
            text="Build a channel just for your child"
            usecase_gif="img/baby_channel.gif"
        />
        <Usecase
            img="img/videotape.svg"
            text="Home videos on demand"
        />
        <Usecase
            img="img/musicparty.svg"
            text="Stream your party mix!"
        />
        <Usecase
            img="img/mortarboard.svg"
            text="Put a graduate compilation on loop"
        />
        <Usecase
            img="img/shop.svg"
            text="Loop your shop's promo video"
        />
    </section>
    <section id="login">
        <form @submit.prevent="form_submit">
            <div>
                <label for="login_user">Username:</label>
                <input type="text" autocomplete="username" id="login_user" v-model="username">
            </div>
            <div>
                <label for="login_pass">Password:</label>
                <input type="password" autocomplete="password" id="login_pass" v-model="password">
            </div>
            <div>
                <input class="bigger_text" type="submit" value="Login">
                <a href="signup.html" class="input_button bigger_text">Register</a>
            </div>
        </form>
    </section>
    <section id="links">
        <h2>Apps</h2>
        <h3>
            <a href="https://channelstore.roku.com/details/707afc86801deb28e35d3984cdc59b68/running-stream">Roku Channel</a>
            <a href="https://player.runningstream.cc">Web Browser Player</a>
        </h3>
        <h2>Documentation</h2>
        <h3>
            <a href="https://docs.runningstream.cc/">Getting Started</a>
        </h3>
    </section>
    <section id="trailer">
        <h2>Free.  No catch.</h2>
        <h3><a href="https://github.com/runningstream/channel_builder">Source Code Here</a></h3>
        <p>Why?  We're nerds and we use this ourselves.  It doesn't cost us much to let others in too.</p>
        <p><a href="mailto:runningstreamllc@gmail.com">Contact Us</a><a href="https://docs.runningstream.cc/privacy_policy/">Privacy Policy</a></p>
    </section>

    <Alert
        text="Login success but validation failure for unknown reason.  Please refresh the page and try to login again."
        :display="alert_login_succ_val_fail" @closeModal="alert_login_succ_val_fail=false"
    />
    <Alert
        text="Login failed."
        :display="alert_login_fail" @closeModal="alert_login_fail=false"
    />
    <Spinner :display="show_spinner" />
</template>

<style scoped>
    div {
        padding: 1em;
    }
    section {
        text-align: center;
    }
    #intro {
        position: relative;
        width: 100%;
        min-height: min-content;
    }
    #intro_text {
        padding: 100px 15px;
        color: var(--color-ltolive);
        text-shadow: 10px 0px 20px black, -10px 0px 20px black;
    }
    #intro h1, #intro h2 {
        margin-block-start: 1em;
        margin-block-end: 1em;
    }
    #intro h2 {
        margin-inline-start: 0;
        margin-inline-start: 0;
    }
    #intro .input_button
    {
        text-transform: uppercase;
        padding: 15px 30px;
        display: inline-block;

        transition-duration: .25s;
    }
    #intro .input_button:hover {
        transform: scale3d(1.1, 1.1, 1.1);
        transition-duration: .25s;
    }
    #intro:before, #intro:after {
        content: "";
        position: absolute;
        top: 0px;
        left: 0px;
        width: 100%;
        height: 100%;
    }
    #intro:before {
        z-index: -2;
        background-color: black;
    }
    #intro:after {
        z-index: -1;
        background-image: url(/img/busy_city.jpg);
        background-position: center;
        background-size: cover;
        filter: blur(1px);
    }

    #intro2 div {
        padding: 20px;
    }
    #intro2 div:first-letter {
        font-size: 2em;
    }

    #login, #links, #usecases, #intro2, #trailer {
        padding: 100px 25px;
    }
    #links, #usecases {
        background-color: var(--color-scarlet);
        color: var(--color-ltolive);
    }

    #usecases {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
    }
    #usecases h1 {
        width: 100%;
        margin: 0 0 10px 0;
    }
    #usecases p {
        width: 100%;
        margin: 0 0 50px 0;
    }
    #usecases .usecase {
        width: 150px;
        margin: 20px;
    }

    #links a {
        display: block;
        padding: .5em;
    }

    #trailer a {
        padding: 20px;
    }
</style>
