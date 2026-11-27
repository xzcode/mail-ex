<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
const props=defineProps<{html:string; disabled:boolean; zh:boolean}>()
const emit=defineEmits<{update:[value:{html:string;body:string}]}>()
const editor=ref<HTMLElement|null>(null)
const allowed=new Set(['P','DIV','SPAN','BR','HR','H1','H2','H3','H4','H5','H6','B','STRONG','I','EM','U','S','STRIKE','DEL','SUB','SUP','BLOCKQUOTE','PRE','CODE','UL','OL','LI','DL','DT','DD','TABLE','CAPTION','THEAD','TBODY','TFOOT','TR','TD','TH'])
const blocked=new Set(['SCRIPT','STYLE','IFRAME','OBJECT','EMBED','SVG','MATH','TEMPLATE','NOSCRIPT','HEAD','TITLE','FORM'])
let lastEmitted:string|null=null
function safeChildren(source:Node,target:Node){
  for(const node of Array.from(source.childNodes)){
    if(node.nodeType===Node.TEXT_NODE){target.appendChild(document.createTextNode(node.textContent||''));continue}
    if(!(node instanceof Element)||blocked.has(node.tagName))continue
    if(allowed.has(node.tagName)){
      const safe=document.createElement(node.tagName.toLowerCase());safeChildren(node,safe);target.appendChild(safe)
    }else safeChildren(node,target)
  }
}
function safeHtml(element:HTMLElement){const box=document.createElement('div');safeChildren(element,box);return box.innerHTML}
function change(){
  if(!editor.value||props.disabled)return
  const html=safeHtml(editor.value),body=editor.value.innerText
  lastEmitted=html;emit('update',{html,body})
}
function load(value:string){
  if(!editor.value||value===lastEmitted)return
  // Only core-sanitized HTML or escaped plain text enters this property.
  const template=document.createElement('template');template.innerHTML=value
  const safe=document.createElement('div');safeChildren(template.content,safe)
  editor.value.replaceChildren(...Array.from(safe.childNodes));lastEmitted=value
}
function format(command:string,value?:string){
  if(!editor.value||props.disabled)return
  editor.value.focus()
  const selection=window.getSelection()
  if(selection&&(!selection.anchorNode||!editor.value.contains(selection.anchorNode))){
    const range=document.createRange();range.selectNodeContents(editor.value);range.collapse(false);selection.removeAllRanges();selection.addRange(range)
  }
  document.execCommand(command,false,value);change()
}
function paste(event:ClipboardEvent){
  event.preventDefault()
  if(props.disabled)return
  document.execCommand('insertText',false,event.clipboardData?.getData('text/plain')||'');change()
}
onMounted(()=>load(props.html))
watch(()=>props.html,load)
</script>
<template>
  <div class="rich-editor">
    <div class="format-toolbar" role="toolbar" :aria-label="zh?'文字格式':'Text formatting'">
      <button type="button" :disabled="disabled" title="Bold" @mousedown.prevent @click="format('bold')"><b>B</b></button>
      <button type="button" :disabled="disabled" title="Italic" @mousedown.prevent @click="format('italic')"><i>I</i></button>
      <button type="button" :disabled="disabled" title="Underline" @mousedown.prevent @click="format('underline')"><u>U</u></button>
      <button type="button" :disabled="disabled" title="Strikethrough" @mousedown.prevent @click="format('strikeThrough')"><s>S</s></button>
      <button type="button" :disabled="disabled" @mousedown.prevent @click="format('formatBlock','h2')">H2</button>
      <button type="button" :disabled="disabled" @mousedown.prevent @click="format('formatBlock','p')">{{zh?'正文':'Text'}}</button>
      <button type="button" :disabled="disabled" @mousedown.prevent @click="format('insertUnorderedList')">{{zh?'项目符号':'Bullets'}}</button>
      <button type="button" :disabled="disabled" @mousedown.prevent @click="format('insertOrderedList')">{{zh?'编号':'Numbering'}}</button>
      <button type="button" :disabled="disabled" @mousedown.prevent @click="format('formatBlock','blockquote')">{{zh?'引用':'Quote'}}</button>
      <button type="button" :disabled="disabled" @mousedown.prevent @click="format('removeFormat')">{{zh?'清除格式':'Clear format'}}</button>
    </div>
    <div ref="editor" class="rich-body safe-mail-html" :contenteditable="!disabled" role="textbox" aria-multiline="true" :aria-label="zh?'邮件正文':'Mail body'" spellcheck="true" @input="change" @paste="paste" @drop.prevent></div>
  </div>
</template>
