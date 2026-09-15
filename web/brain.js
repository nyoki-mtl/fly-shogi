'use strict';
// Coordinates and activity share atlas order. Camera motion is decorative;
// Neural glow uses recorded model spikes. Nectar and gold feedback are explanatory animation.
class BrainView {
  constructor(canvas) {
    this.canvas=canvas;this.ctx=canvas.getContext('2d');this.nodes=[];this.data=null;
    this.started=0;this.duration=2000;this.rewardAt=-Infinity;this.angle=0;this.dragging=false;
    this.reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
    this.sprite={};
    for(const [key,color] of Object.entries({kc:'153,228,200',mbon:'243,153,124',other:'103,155,154',reward:'239,196,110'})){
      const c=document.createElement('canvas');c.width=c.height=48;const g=c.getContext('2d');
      const gradient=g.createRadialGradient(24,24,0,24,24,24);gradient.addColorStop(0,`rgba(${color},1)`);gradient.addColorStop(.16,`rgba(${color},.85)`);gradient.addColorStop(.42,`rgba(${color},.2)`);gradient.addColorStop(1,`rgba(${color},0)`);g.fillStyle=gradient;g.fillRect(0,0,48,48);this.sprite[key]=c;
    }
    canvas.addEventListener('pointerdown',e=>{this.dragging=true;this.lastX=e.clientX;canvas.setPointerCapture(e.pointerId);});
    canvas.addEventListener('pointermove',e=>{if(this.dragging){this.angle+=(e.clientX-this.lastX)*.006;this.lastX=e.clientX;}});
    canvas.addEventListener('pointerup',()=>{this.dragging=false;});canvas.addEventListener('pointercancel',()=>{this.dragging=false;});
    new ResizeObserver(()=>this.resize()).observe(canvas);this.resize();
    this.ready=fetch('/api/atlas').then(r=>{if(!r.ok)throw Error('Unable to load brain coordinates.');return r.json();}).then(atlas=>{
      this.nodes=atlas.nodes;document.getElementById('brain-empty').hidden=true;
      document.getElementById('brain').setAttribute('aria-label',`${this.nodes.length.toLocaleString()} MaleCNS soma positions. Drag to rotate. Neural glow shows model spike counts; gold shows external reward feedback.`);
      return true;
    }).catch(error=>{document.getElementById('brain-empty').textContent=error.message;return false;});
    const frame=now=>{this.draw(now);requestAnimationFrame(frame);};requestAnimationFrame(frame);
  }
  resize(){const box=this.canvas.getBoundingClientRect();const dpr=Math.min(devicePixelRatio||1,2);this.canvas.width=Math.round(box.width*dpr);this.canvas.height=Math.round(box.height*dpr);this.ctx.setTransform(dpr,0,0,dpr,0,0);this.width=box.width;this.height=box.height;}
  play(data,duration){this.data=data;this.duration=duration;this.started=performance.now();this.max=Math.max(1,...data.brain_activity.flat());}
  clear(){this.data=null;this.rewardAt=-Infinity;this.rewardAmount=0;}
  reward(amount){this.rewardAmount=amount;this.rewardAt=performance.now();}
  feedbackPhase(now){
    const elapsed=now-this.rewardAt;
    const amount=this.rewardAmount||0;
    const fill=Math.max(0,Math.min(1,elapsed/700));
    return {elapsed,amount,fill:amount*fill,travel:Math.max(0,Math.min(1,(elapsed-700)/650)),wave:(elapsed-1350)/1600};
  }
  drawNectar(g,w,h,now,cx,cy){
    const f=this.feedbackPhase(now),unit=Math.min(w/640,h/520),cw=82*unit,ch=40*unit,ny=h*.91;
    g.save();g.translate(cx,ny);
    // A fixed-width reservoir makes nectar level proportional to external reward.
    g.fillStyle='#efc46e08';g.fillRect(-cw/2,-ch,cw,ch);
    const nectar=g.createLinearGradient(0,-ch,0,0);nectar.addColorStop(0,'#ffd784');nectar.addColorStop(1,'#c68324');
    g.fillStyle=nectar;g.fillRect(-cw/2,-ch*f.fill,cw,ch*f.fill);
    if(f.fill>0){g.strokeStyle='#ffe6a3';g.lineWidth=unit;g.beginPath();g.moveTo(-cw/2,-ch*f.fill);g.lineTo(cw/2,-ch*f.fill);g.stroke();}
    g.strokeStyle='#bca26c80';g.lineWidth=1.5*unit;g.beginPath();g.moveTo(-cw/2,-ch-5*unit);g.lineTo(-cw/2,0);g.lineTo(cw/2,0);g.lineTo(cw/2,-ch-5*unit);g.stroke();
    if(f.amount>0&&f.elapsed>=0&&f.elapsed<700){
      const t=f.elapsed/700,r=22*unit*Math.sqrt(f.amount);
      g.translate(0,-ch-54*unit*(1-t));g.scale(r,r);
      g.beginPath();g.moveTo(0,-1.4);g.bezierCurveTo(.3,-.7,1,.05,1,.5);g.bezierCurveTo(1,1.65,-1,1.65,-1,.5);g.bezierCurveTo(-1,.05,-.3,-.7,0,-1.4);
      g.fillStyle='#f6c866';g.fill();g.restore();g.save();
    }else{g.restore();g.save();}
    if(f.amount>0&&f.elapsed>=700&&f.elapsed<1350){
      const t=f.travel,fromY=ny-ch,toY=cy;
      g.globalCompositeOperation='lighter';g.strokeStyle=`rgba(239,196,110,${f.amount*.5})`;g.lineWidth=2*unit;
      g.beginPath();g.moveTo(cx,fromY);g.lineTo(cx,fromY+(toY-fromY)*t);g.stroke();
      for(let i=0;i<5;i++){const at=Math.max(0,t-i*.035),y=fromY+(toY-fromY)*at,size=(15-i*2)*unit;
        g.globalAlpha=f.amount*(1-i*.15);g.drawImage(this.sprite.reward,cx-size/2,y-size/2,size,size);}
    }
    g.restore();
  }
  draw(now){
    const g=this.ctx,w=this.width,h=this.height;g.clearRect(0,0,w,h);if(!this.nodes.length)return;
    // A callback's frame timestamp can precede play() in the same animation frame.
    const t=this.data?Math.max(0,Math.min(1,(now-this.started)/this.duration)):0;
    const bins=this.data?.brain_activity;const at=t*Math.max(0,(bins?.length||1)-1),lo=Math.floor(at),hi=Math.min(lo+1,(bins?.length||1)-1),blend=at-lo;
    const decay=this.data?Math.max(.28,1-Math.max(0,now-this.started-this.duration)/1400):0;
    const turn=this.angle+(this.reduced?0:Math.sin(now/10000)*.13),ca=Math.cos(turn),sa=Math.sin(turn);
    const scale=Math.min(w*.44,h*.61),cx=w*.50,cy=h*.42;
    const rewardAge=this.feedbackPhase(now).wave,reward=rewardAge>=0&&rewardAge<1?Math.sin(rewardAge*Math.PI)*(this.rewardAmount||0):0;
    g.save();g.globalCompositeOperation='lighter';
    for(let i=0;i<this.nodes.length;i++){
      const node=this.nodes[i],[x,y,z]=node.p;const depth=z*ca-x*sa;
      const px=cx+(x*ca+z*sa)*scale,py=cy+y*scale*.98;
      const value=bins?(bins[lo][i]*(1-blend)+bins[hi][i]*blend):0;
      const level=Math.sqrt(value/(this.max||1))*decay;
      const bright=node.group==='kc'||node.group==='mbon';
      const r=(bright?1.3:1.0)+level*2.8;
      g.globalAlpha=(bright?.20:.12)+(depth+1)*.04;g.fillStyle=bright?'#96cbbb':'#638787';g.beginPath();g.arc(px,py,r*.65,0,Math.PI*2);g.fill();
      if(level>.01){g.globalAlpha=Math.min(1,level*.9+.05);const size=5+level*17;g.drawImage(this.sprite[node.group],px-size/2,py-size/2,size,size);}
      if(reward&&bright){const distance=Math.hypot(x,y);const band=Math.exp(-Math.pow((distance-rewardAge*.95)*8,2))*reward;g.globalAlpha=band*.8;if(band>.02)g.drawImage(this.sprite.reward,px-10,py-10,20,20);}
    }
    if(reward){g.globalAlpha=reward*.65;g.strokeStyle='#efc46e';g.lineWidth=1;g.beginPath();g.ellipse(cx,cy,scale*rewardAge,scale*rewardAge*.65,0,0,Math.PI*2);g.stroke();}
    g.restore();
    this.drawNectar(g,w,h,now,cx,cy);
    this.currentMs=this.data?this.data.observation_start_ms+t*(this.data.simulation_ms-this.data.observation_start_ms):0;
  }
}
