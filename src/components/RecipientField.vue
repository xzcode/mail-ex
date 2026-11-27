<script setup lang="ts">
import {computed,ref} from 'vue'
import type {Contact} from '../api'
const props=defineProps<{modelValue:string;contacts:Contact[];required?:boolean;placeholder?:string;zh:boolean}>()
const emit=defineEmits<{ 'update:modelValue':[value:string] }>()
const focused=ref(false),highlight=ref(-1)
const tail=computed(()=>props.modelValue.slice(props.modelValue.lastIndexOf(',')+1).trim().toLowerCase())
const options=computed(()=>{
  const q=tail.value
  const earlier=props.modelValue.slice(0,Math.max(0,props.modelValue.lastIndexOf(','))).split(',').map(s=>s.trim().toLowerCase())
  return props.contacts.filter(c=>!earlier.includes(c.email.toLowerCase())&&(!q||[c.email,c.name,c.group_name].some(s=>s.toLowerCase().includes(q))))
})
function choose(c:Contact){
  const split=props.modelValue.lastIndexOf(',')
  emit('update:modelValue',(split<0?'':props.modelValue.slice(0,split+1)+' ')+c.email)
  focused.value=false;highlight.value=-1
}
function change(e:Event){focused.value=true;highlight.value=-1;emit('update:modelValue',(e.target as HTMLInputElement).value)}
function keyboard(e:KeyboardEvent){
  if(e.key==='Escape'){focused.value=false;highlight.value=-1;return}
  if(!focused.value||!options.value.length)return
  if(e.key==='ArrowDown'){e.preventDefault();highlight.value=(highlight.value+1)%options.value.length}
  if(e.key==='ArrowUp'){e.preventDefault();highlight.value=(highlight.value-1+options.value.length)%options.value.length}
  if(e.key==='Enter'&&highlight.value>=0){e.preventDefault();const c=options.value[highlight.value];if(c)choose(c)}
}
</script>
<template>
  <div class="recipient-field">
    <input :value="modelValue" :required="required" :placeholder="placeholder" autocomplete="off" @input="change" @focus="focused=true;highlight=-1" @blur="focused=false" @keydown="keyboard" :aria-expanded="focused&&options.length>0">
    <div v-if="focused&&options.length" class="recipient-options" role="listbox">
      <button v-for="(c,i) in options" :key="c.id" type="button" role="option" :aria-selected="i===highlight" :class="{active:i===highlight}" @mousedown.prevent @click="choose(c)"><strong>{{c.name||c.email}}</strong><small>{{c.email}}<template v-if="c.group_name"> · {{c.group_name}}</template></small></button>
    </div>
  </div>
</template>
