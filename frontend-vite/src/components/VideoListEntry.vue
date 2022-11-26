<script setup lang="ts">
    import { ref } from "vue";
    import VideoList from "./VideoList.vue";
    import MediaEdit from "./VideoListEntry/MediaEdit.vue";
    import SublistEdit from "./VideoListEntry/SublistEdit.vue";
    import Confirm from "./VideoListEntry/Confirm.vue";
    import RenameChannel from "./VideoListEntry/RenameChannel.vue";

    import type { Ref } from "vue";
    import type { VideoType } from "../api_js/serverAPI";

    const params = defineProps({
        entry: {
            type: Object as () => VideoType,
            required: true,
        },
        firstEntry: Boolean,
        lastEntry: Boolean,
        depth: {
            type: Number,
            default: 0,
        },
        editEnable: Boolean,
    });

    const emit = defineEmits<{
        (e: "videoSelected", entry: VideoType) : void
        (e: "updateEntry", entry: VideoType) : void
        (e: "deleteEntry", entry: VideoType) : void
        (e: "moveUpEntry", entry: VideoType) : void
        (e: "moveDownEntry", entry: VideoType) : void
        (e: "renameChannel", cur_name: string | undefined, new_name: string) : void
        (e: "deleteChannel", name: string | undefined) : void
        (e: "setActiveChannel", name: string) : void
    }>();

    const show_edit_media: Ref<boolean> = ref(false);
    const show_add_media: Ref<boolean> = ref(false);
    const show_edit_sublist: Ref<boolean> = ref(false);
    const show_add_sublist: Ref<boolean> = ref(false);
    const show_rename_channel: Ref<boolean> = ref(false);
    const show_delete_channel: Ref<boolean> = ref(false);
    const show_delete_entry: Ref<boolean> = ref(false);

    function select_video( entry: VideoType ) : void {
        if( entry.type == "video" ) {
            emit("videoSelected", entry);
        }
    }

    function update_entries( new_entries: Array<VideoType> ) : void {
        let output = JSON.parse(JSON.stringify(params.entry));
        output.entries = new_entries;
        emit("updateEntry", output);
    }

    function add_entry( new_entry: VideoType ) : void {
        if( params.entry.entries != undefined ) {
            let output = JSON.parse(JSON.stringify(params.entry));
            output.entries.push(new_entry);
            emit("updateEntry", output);
        }
    }
</script>

<template>
    <div class="vidlistentry" :class="'vidListEntDepth' + depth" @click="select_video(entry)">
        <div v-if="['video', 'audio'].includes(entry.type == undefined ? '' : entry.type)">
            <div>{{ entry.name }}</div>
            <div class="entryinfoarea">
                <img :src="entry.image" v-if="entry.image">
                <div class="buttonarea" v-if="editEnable">
                    <input type="button" value="Modify" @click="show_edit_media=true">
                    <input type="button" value="Delete" @click="show_delete_entry=true">
                    <input type="button" value="Move Up"
                        v-if="!firstEntry" @click="$emit('moveUpEntry', entry)">
                    <input type="button" value="Move Down"
                        v-if="!lastEntry" @click="$emit('moveDownEntry', entry)">
                </div>
            </div>
        </div>
        <details v-if="entry.type == 'sublist'">
            <summary>
                <span>{{ entry.name }}</span>
                <div class="entryinfoarea">
                    <img :src="entry.image" v-if="entry.image">
                    <div class="buttonarea" v-if="editEnable">
                        <input type="button" value="Modify" @click="show_edit_sublist=true">
                        <input type="button" value="Delete" @click="show_delete_entry=true">
                        <input type="button" value="Add Sublist" @click="show_add_sublist=true">
                        <input type="button" value="Add Media" @click="show_add_media=true">
                        <input type="button" value="Move Up"
                            v-if="!firstEntry" @click="$emit('moveUpEntry', entry)">
                        <input type="button" value="Move Down"
                            v-if="!lastEntry" @click="$emit('moveDownEntry', entry)">
                    </div>
                </div>
            </summary>
            <VideoList
                :channelList="entry.entries" :depth="depth"
                :editEnable="editEnable"
                @videoSelected="$emit('videoSelected', $event)"
                @updateEntries="update_entries($event)"
            />
        </details>
        <div v-if="entry.type == 'toplevel'">
            <div>
                <span>{{ entry.name }}</span>
                <div class="entryinfoarea">
                    <div class="buttonarea" v-if="editEnable">
                        <input type="button" value="Add Sublist" @click="show_add_sublist=true">
                        <input type="button" value="Add Media" @click="show_add_media=true">
                        <input type="button" value="Rename" @click="show_rename_channel=true">
                        <input type="button" value="Delete" @click="show_delete_channel=true">
                        <input type="button" value="Set Active" @click="$emit('setActiveChannel', entry.name)">
                    </div>
                </div>
            </div>
            <VideoList
                :channelList="entry.entries"
                :editEnable="editEnable"
                @videoSelected="$emit('videoSelected', $event)"
                @updateEntries="update_entries($event)"
            />
        </div>
        <MediaEdit v-if="editEnable"
            :inputEntry="entry" :display="show_edit_media"
            @cancelModal="show_edit_media=false"
            @saveModal="show_edit_media=false; $emit('updateEntry', $event);"
        />
        <MediaEdit v-if="editEnable"
            :display="show_add_media"
            @cancelModal="show_add_media=false"
            @saveModal="show_add_media=false; add_entry($event);"
        />
        <SublistEdit v-if="editEnable"
            :inputEntry="entry" :display="show_edit_sublist"
            @cancelModal="show_edit_sublist=false"
            @saveModal="show_edit_sublist=false; $emit('updateEntry', $event);"
        />
        <SublistEdit v-if="editEnable"
            :display="show_add_sublist"
            @cancelModal="show_add_sublist=false"
            @saveModal="show_add_sublist=false; add_entry($event);"
        />
        <RenameChannel v-if="editEnable"
            :display="show_rename_channel" :currentName="entry.name"
            @cancelModal="show_rename_channel=false"
            @saveModal="show_rename_channel=false; $emit('renameChannel', entry.name, $event);"
        />
        <Confirm v-if="editEnable"
            :display="show_delete_channel"
            :text="`Are you sure you want to delete channel: ${ entry.name }`"
            @cancelModal="show_delete_channel=false"
            @okModal="show_delete_channel=false; $emit('deleteChannel', entry.name);"
        />
        <Confirm v-if="editEnable"
            :display="show_delete_entry"
            :text="`Are you sure you want to delete entry: ${ entry.name }`"
            @cancelModal="show_delete_entry=false"
            @okModal="show_delete_entry=false; $emit('deleteEntry', entry);"
        />
    </div>
</template>

<style scoped>
    details > summary {
        list-style-type: "+ ";
    }
    details[open] > summary {
        list-style-type: "- ";
    }

    div {
        text-align: left;
    }

    .vidlistentry {
        padding: 5px;
        padding-bottom: 10px;
        position: relative;
    }
    .vidlistentry:not(:last-child):before {
        content: "";
        position: absolute;
        left: 5%;
        bottom: 0;
        height: 1px;
        width: 90%;
    }

    .vidListEntDepth0.vidlistentry:not(:last-child):before {
        border-bottom: 1px var(--color-teal) solid;
    }
    .vidListEntDepth1.vidlistentry:not(:last-child):before {
        border-bottom: 1px var(--color-ltolive) solid;
    }

    .entryinfoarea {
        display: flex;
        flex-wrap: wrap;
    }

    img {
        display: block;
        max-width: 90%;
        width: 100px;
        margin-bottom: auto;
        border-radius: 10px;
    }

    .buttonarea {
        display: flex;
        flex-wrap: wrap;
        flex: 1 0 50px;
    }

    .buttonarea input {
        margin-bottom: auto;
    }
</style>
