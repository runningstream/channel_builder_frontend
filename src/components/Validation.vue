<script setup lang="ts">
    import { ref } from "vue";
    import type { Ref } from "vue";

    import { apiValidateAccount } from "../api_js/serverAPI";
    import { jump_to_login } from "./Helpers";
    import Alert from "./Alert.vue";
    import Spinner from "./Spinner.vue";

    const alert_val_success: Ref<boolean> = ref(false);
    const alert_inval_code: Ref<boolean> = ref(false);
    const alert_val_fail: Ref<boolean> = ref(false);
    const show_spinner: Ref<boolean> = ref(false);

    function validate_attempt(_ev: Event) {
        const urlParams = new URLSearchParams(window.location.search);
        const val_code = urlParams.get("val_code");

        if( val_code == null ) {
            alert_inval_code.value = true;
            return;
        }

        show_spinner.value = true;
        apiValidateAccount(val_code)
            .then( () => {
                show_spinner.value = false;
                alert_val_success.value = true;
            })
            .catch( (error: any) => {
                show_spinner.value = false;
                if( error.response && error.response.status == 403 ) {
                    alert_inval_code.value = true;
                } else {
                    alert_val_fail.value = true;
                }
            });
    }
</script>

<template>
    <div>
        <section>
            <h1>Welcome to Running Stream!</h1>
        </section>
        <section>
            <div>Register here! If you didn't request to register, or you no longer wish to receve registration emails, just delete the one you received. You will not receive any further communications from this service.</div>
            <input type="button" class="bigger_text" value="Validate Account" @click.prevent="validate_attempt">
        </section>

        <Alert
            text="User validation successful!  Now log in."
            :display="alert_val_success" @closeModal="alert_val_success=false; jump_to_login();"
        />
        <Alert
            text="Invalid validation code!"
            :display="alert_inval_code" @closeModal="alert_inval_code=false"
        />
        <Alert
            text="Validation failed."
            :display="alert_val_fail" @closeModal="alert_val_fail=false"
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
    input[type="button"] {
        margin: 1.33em 0;
    }
</style>
