<script setup lang="ts">
    import { ref } from "vue";
    import type { Ref } from "vue";

    import { apiCreateAccount } from "../api_js/serverAPI";
    import { jump_to_login } from "./Helpers";
    import Alert from "./Alert.vue";
    import Spinner from "./Spinner.vue";

    const username: Ref<string> = ref("");
    const password: Ref<string> = ref("");
    const password_verify: Ref<string> = ref("");

    const alert_no_pw_match: Ref<boolean> = ref(false);
    const alert_pw_short: Ref<boolean> = ref(false);
    const alert_acct_requested: Ref<boolean> = ref(false);
    const alert_acct_failed: Ref<boolean> = ref(false);
    const show_spinner: Ref<boolean> = ref(false);

    function register_attempt(_ev: Event) : void {
        if( password.value != password_verify.value ) {
            alert_no_pw_match.value = true;
            return;
        }
        if( password.value.length <= 5 ) {
            alert_pw_short.value = true;
            return;
        }

        show_spinner.value = true;
        apiCreateAccount(username.value, password.value)
            .then( () => {
                show_spinner.value = false;
                alert_acct_requested.value = true;
            })
            .catch( (_error: any) => {
                show_spinner.value = false;
                alert_acct_failed.value = true;
            });
    }
</script>

<template>
    <div>
        <section>
            <h1>Sign up now!</h1>
        </section>
        <section>
            <form @submit.prevent="register_attempt">
                <div>
                    <label for="reg_username">Email Address:</label>
                    <input type="text" id="reg_username" autocomplete="username" v-model="username">
                </div>
                <div>
                    <label for="reg_password">Password:</label>
                    <input type="password" id="reg_password" autocomplete="current-password" v-model="password">
                </div>
                <div>
                    <label for="verify_password">Verify Password:</label>
                    <input type="password" id="verify_password" autocomplete="current-password" v-model="password_verify">
                </div>
                <input type="submit" class="bigger_text" value="Register">
            </form>
            <div id="pw_alert">You will need to enter this password on your streaming device later, so choose one that won't be too difficult.</div>
        </section>
        <section>
            <p><a href="https://docs.runningstream.cc/privacy_policy/">Full Privacy Policy</a> - in short, we will never give anyone else your information.  You will only ever receive a signup confirmation email from us.</p>
        </section>

        <Alert
            text="The passwords do not match - they must match."
            :display="alert_no_pw_match" @closeModal="alert_no_pw_match=false"
        />
        <Alert
            text="The password was too short, it must be longer than 5 characters."
            :display="alert_pw_short" @closeModal="alert_pw_short=false"
        />
        <Alert
            text="User account requested, look for an email..."
            :display="alert_acct_requested" @closeModal="alert_acct_requested=false; jump_to_login();"
        />
        <Alert
            text="User account creation failed.  Perhaps the email address is already registered."
            :display="alert_acct_failed" @closeModal="alert_acct_failed=false"
        />
        <Spinner :display="show_spinner" />
    </div>
</template>

<style scoped>
    section {
        text-align: center;
        max-width: 80%;
        margin: auto;
    }
    #pw_alert {
        margin: 1.33em 0;
        font-weight: bold;
    }
</style>
