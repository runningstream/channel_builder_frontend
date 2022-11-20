<script setup lang="ts">
    const props = defineProps(["channelList", "activeChannel", "channelSelected"]);
    const emit = defineEmits(["channelSelected"]);

    function channel_selected(channel: string) {
        emit("channelSelected", channel);
    }
</script>

<template>
    <div id="channel_list">
        <div class="channel_list_entry"
            v-for="channel in channelList"
            :class="{ 'entry_sel': channel == channelSelected, active_channel : channel == activeChannel }"
            @click="channel_selected(channel)"
        >
            {{ channel }}
        </div>
    </div>
</template>

<style scoped>
    .channel_list_entry:nth-child(odd) {
        background: var(--color-teal);
    }
    .channel_list_entry:nth-child(even) {
        background: var(--color-ltolive);
    }

    .channel_list_entry {
        cursor: pointer;
        border: 1px var(--color-ltolive) solid;
        border-radius: 20px;
        padding: 20px;
        text-align: center;
    }

    .entry_sel {
        border: 1px var(--color-dkscarlet) solid;
    }

    .active_channel {
        font-style: italic;
        font-weight: bold;
    }
</style>
