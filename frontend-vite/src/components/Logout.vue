<script setup lang="ts">
    import { ref } from "vue";
    import type { Ref } from "vue";

    import { apiLogout } from "../api_js/serverAPI";
    import { jump_to_login } from "./Helpers";

    import Alert from "./Alert.vue";
    import Spinner from "./Spinner.vue";

    const show_spinner: Ref<boolean> = ref(false);
    const alert_api_error_show: Ref<boolean> = ref(false);
    const alert_api_error_text: Ref<string> = ref("");

    function logout() {
        show_spinner.value = true;

        apiLogout()
            .then( jump_to_login )
            .catch( (error: any) => {
                show_spinner.value = false;
                alert_api_error_text.value = `Logout failure: ${error}`;
                alert_api_error_show.value = true;
            });
    }
</script>

<template>
    <div class="logout-button" @click="logout()" title="Logout">
        <Alert :text="alert_api_error_text"
            :display="alert_api_error_show" @closeModal="alert_api_error_show=false"
        />
        <Spinner :display="show_spinner" />
    </div>
</template>

<style scoped>
    .logout-button {
        background-image: url(/img/logout.svg);
    }
</style>
